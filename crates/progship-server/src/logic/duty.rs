//! Pure duty scheduling logic — shift timing, duty fitness, sleep scheduling.

use crate::tables::shifts;

/// Check if a crew member should be on duty based on shift and hour.
pub fn should_be_on_duty(shift: u8, hour: f32) -> bool {
    match shift {
        shifts::ALPHA => (6.0..14.0).contains(&hour),
        shifts::BETA => (14.0..22.0).contains(&hour),
        shifts::GAMMA => !(6.0..22.0).contains(&hour),
        _ => false,
    }
}

/// Check if a crew member is fit for duty based on their needs.
///
/// Exhausted, starving, or critically injured crew should skip duty.
/// Returns true if fit enough to work.
pub fn is_fit_for_duty(hunger: f32, fatigue: f32, health: f32) -> bool {
    // Can't work if critically fatigued, starving, or badly injured
    fatigue < 0.9 && hunger < 0.9 && health > 0.2
}

/// Determine the optimal sleep window for a crew member based on their shift.
///
/// Returns (sleep_start_hour, sleep_end_hour) in 24h format.
/// Crew should sleep during the 8-hour block furthest from their shift.
pub fn crew_sleep_window(shift: u8) -> (f32, f32) {
    match shift {
        shifts::ALPHA => (22.0, 6.0),  // Alpha works 6-14, sleeps 22-06
        shifts::BETA => (6.0, 14.0), // Beta works 14-22, sleeps 06-14 (but overlaps, so really 2-10 is better)
        shifts::GAMMA => (14.0, 22.0), // Gamma works 22-06, sleeps 14-22
        _ => (22.0, 6.0),
    }
}

/// Check if it's sleep time for a crew member based on their shift.
pub fn is_crew_sleep_time(shift: u8, hour: f32) -> bool {
    let (start, end) = crew_sleep_window(shift);
    if start < end {
        (start..end).contains(&hour)
    } else {
        // Wraps midnight
        !(end..start).contains(&hour)
    }
}

/// Check if it's sleep time for a passenger (non-crew).
pub fn is_passenger_sleep_time(hour: f32) -> bool {
    !(6.0..22.0).contains(&hour)
}

/// Determine if a crew member should sleep now based on shift, fatigue, and time.
pub fn should_sleep(shift: u8, hour: f32, fatigue: f32) -> bool {
    // Very tired — sleep regardless of schedule
    if fatigue > 0.85 {
        return true;
    }
    // Moderate fatigue during sleep window — go to bed
    if fatigue > 0.5 && is_crew_sleep_time(shift, hour) {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_be_on_duty_alpha() {
        assert!(should_be_on_duty(shifts::ALPHA, 6.0));
        assert!(should_be_on_duty(shifts::ALPHA, 10.0));
        assert!(should_be_on_duty(shifts::ALPHA, 13.9));
        assert!(!should_be_on_duty(shifts::ALPHA, 5.9));
        assert!(!should_be_on_duty(shifts::ALPHA, 14.0));
    }

    #[test]
    fn test_should_be_on_duty_beta() {
        assert!(should_be_on_duty(shifts::BETA, 14.0));
        assert!(should_be_on_duty(shifts::BETA, 18.0));
        assert!(!should_be_on_duty(shifts::BETA, 13.9));
        assert!(!should_be_on_duty(shifts::BETA, 22.0));
    }

    #[test]
    fn test_should_be_on_duty_gamma() {
        assert!(should_be_on_duty(shifts::GAMMA, 22.0));
        assert!(should_be_on_duty(shifts::GAMMA, 0.0));
        assert!(should_be_on_duty(shifts::GAMMA, 5.9));
        assert!(!should_be_on_duty(shifts::GAMMA, 6.0));
        assert!(!should_be_on_duty(shifts::GAMMA, 21.9));
    }

    #[test]
    fn test_is_fit_for_duty() {
        assert!(is_fit_for_duty(0.5, 0.5, 0.8));
        assert!(!is_fit_for_duty(0.95, 0.5, 0.8)); // starving
        assert!(!is_fit_for_duty(0.5, 0.95, 0.8)); // exhausted
        assert!(!is_fit_for_duty(0.5, 0.5, 0.1)); // critically injured
                                                  // Edge cases
        assert!(is_fit_for_duty(0.89, 0.89, 0.21));
        assert!(!is_fit_for_duty(0.9, 0.5, 0.8));
    }

    #[test]
    fn test_crew_sleep_window_alpha() {
        // Alpha works 6-14, sleeps 22-06
        let (start, end) = crew_sleep_window(shifts::ALPHA);
        assert!((start - 22.0).abs() < 0.01);
        assert!((end - 6.0).abs() < 0.01);
    }

    #[test]
    fn test_is_crew_sleep_time_alpha() {
        assert!(is_crew_sleep_time(shifts::ALPHA, 23.0));
        assert!(is_crew_sleep_time(shifts::ALPHA, 0.0));
        assert!(is_crew_sleep_time(shifts::ALPHA, 3.0));
        assert!(!is_crew_sleep_time(shifts::ALPHA, 10.0));
        assert!(!is_crew_sleep_time(shifts::ALPHA, 15.0));
    }

    #[test]
    fn test_is_crew_sleep_time_gamma() {
        // Gamma works 22-06, sleeps 14-22
        assert!(is_crew_sleep_time(shifts::GAMMA, 14.0));
        assert!(is_crew_sleep_time(shifts::GAMMA, 18.0));
        assert!(!is_crew_sleep_time(shifts::GAMMA, 23.0));
        assert!(!is_crew_sleep_time(shifts::GAMMA, 10.0));
    }

    #[test]
    fn test_should_sleep_very_tired() {
        // Very tired — sleep regardless of schedule
        assert!(should_sleep(shifts::ALPHA, 10.0, 0.9));
    }

    #[test]
    fn test_should_sleep_moderate_in_window() {
        // Moderate fatigue during sleep window — sleep
        assert!(should_sleep(shifts::ALPHA, 23.0, 0.6));
    }

    #[test]
    fn test_should_not_sleep_moderate_outside_window() {
        // Moderate fatigue outside sleep window — don't sleep
        assert!(!should_sleep(shifts::ALPHA, 10.0, 0.6));
    }

    #[test]
    fn test_should_not_sleep_low_fatigue() {
        // Not tired — don't sleep
        assert!(!should_sleep(shifts::ALPHA, 23.0, 0.3));
    }
}
