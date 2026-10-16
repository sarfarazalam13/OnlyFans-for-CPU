use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThermalState {
    Normal,
    Elevated,
    Emergency,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SafetyFault {
    TelemetryStale,
    SensorInvalid,
    ThermalEmergency,
    LeaseExpired,
    RestoreFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlLease {
    pub id: Uuid,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafetyState {
    pub writes_enabled: bool,
    pub lease: Option<ControlLease>,
    pub telemetry_fresh: bool,
    pub thermal_state: ThermalState,
    pub active_fault: Option<SafetyFault>,
    pub last_restoration_result: Option<String>,
}

impl Default for SafetyState {
    fn default() -> Self {
        Self {
            writes_enabled: false,
            lease: None,
            telemetry_fresh: false,
            thermal_state: ThermalState::Unknown,
            active_fault: None,
            last_restoration_result: None,
        }
    }
}

impl SafetyState {
    pub fn issue_lease(&mut self, now: DateTime<Utc>, ttl: Duration) -> Option<ControlLease> {
        if !self.writes_enabled
            || !self.telemetry_fresh
            || self.thermal_state != ThermalState::Normal
        {
            return None;
        }
        let lease = ControlLease {
            id: Uuid::new_v4(),
            issued_at: now,
            expires_at: now + ttl,
        };
        self.lease = Some(lease.clone());
        Some(lease)
    }
    pub fn evaluate(&mut self, now: DateTime<Utc>) {
        if self
            .lease
            .as_ref()
            .is_some_and(|lease| lease.expires_at <= now)
        {
            self.lease = None;
            self.active_fault = Some(SafetyFault::LeaseExpired);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CurvePoint {
    pub temperature_c: f32,
    pub speed_percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FanCurve {
    pub points: Vec<CurvePoint>,
    pub hysteresis_c: f32,
    pub minimum_dwell_seconds: u64,
}

impl FanCurve {
    pub fn validate(&self, minimum_speed: u8) -> Result<(), &'static str> {
        if self.points.len() < 2 {
            return Err("curve requires at least two points");
        }
        if self.hysteresis_c < 0.0 || self.minimum_dwell_seconds == 0 {
            return Err("invalid hysteresis or dwell time");
        }
        for pair in self.points.windows(2) {
            if pair[0].temperature_c >= pair[1].temperature_c
                || pair[0].speed_percent > pair[1].speed_percent
            {
                return Err("points must increase monotonically");
            }
        }
        if self
            .points
            .iter()
            .any(|point| point.speed_percent < minimum_speed || point.speed_percent > 100)
        {
            return Err("speed is outside the safe range");
        }
        Ok(())
    }
    pub fn interpolate(&self, temperature_c: f32) -> u8 {
        if temperature_c <= self.points[0].temperature_c {
            return self.points[0].speed_percent;
        }
        for pair in self.points.windows(2) {
            if temperature_c <= pair[1].temperature_c {
                let ratio = (temperature_c - pair[0].temperature_c)
                    / (pair[1].temperature_c - pair[0].temperature_c);
                return (pair[0].speed_percent as f32
                    + ratio * (pair[1].speed_percent - pair[0].speed_percent) as f32)
                    .round() as u8;
            }
        }
        self.points.last().unwrap().speed_percent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn curve_interpolates_and_validates() {
        let curve = FanCurve {
            points: vec![
                CurvePoint {
                    temperature_c: 40.0,
                    speed_percent: 30,
                },
                CurvePoint {
                    temperature_c: 80.0,
                    speed_percent: 90,
                },
            ],
            hysteresis_c: 3.0,
            minimum_dwell_seconds: 5,
        };
        assert!(curve.validate(25).is_ok());
        assert_eq!(curve.interpolate(60.0), 60);
    }
    #[test]
    fn expired_lease_fails_closed() {
        let now = Utc::now();
        let mut state = SafetyState {
            writes_enabled: true,
            telemetry_fresh: true,
            thermal_state: ThermalState::Normal,
            ..Default::default()
        };
        assert!(state.issue_lease(now, Duration::seconds(1)).is_some());
        state.evaluate(now + Duration::seconds(2));
        assert_eq!(state.active_fault, Some(SafetyFault::LeaseExpired));
    }
}
