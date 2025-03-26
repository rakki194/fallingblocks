#[cfg(test)]
mod tests {
    use crate::game::*;

    #[test]
    fn test_board_dimensions() {
        // Verify the standard dimensions of a Tetris board
        assert_eq!(BOARD_WIDTH, 10);
        assert_eq!(BOARD_HEIGHT, 20);
    }

    #[test]
    fn test_scoring_constants() {
        // Verify scoring values are correctly defined
        assert_eq!(POINTS_SINGLE, 40);
        assert_eq!(POINTS_DOUBLE, 100);
        assert_eq!(POINTS_TRIPLE, 300);
        assert_eq!(POINTS_TETRIS, 1200);

        // Check advanced scoring
        assert_eq!(PERFECT_CLEAR_BONUS, 3000);
        assert_eq!(SOFT_DROP_POINTS, 1);
        assert_eq!(HARD_DROP_POINTS, 2);

        // Check T-spin bonuses
        assert_eq!(TSPIN_SINGLE, 800);
        assert_eq!(TSPIN_DOUBLE, 1200);
        assert_eq!(TSPIN_TRIPLE, 1600);
    }

    #[test]
    fn test_level_progression() {
        // Test level progression constants
        assert_eq!(LINES_PER_LEVEL, 10);
        assert_eq!(MAX_LEVEL, 30);
        assert_eq!(STARTING_LEVEL, 1);

        // Verify level thresholds are in ascending order
        for i in 1..LEVEL_SCORE_THRESHOLDS.len() {
            let (prev_score, prev_level) = LEVEL_SCORE_THRESHOLDS[i - 1];
            let (curr_score, curr_level) = LEVEL_SCORE_THRESHOLDS[i];

            // Scores should increase for each threshold
            assert!(curr_score > prev_score);

            // Levels should increase monotonically
            assert!(curr_level > prev_level);
        }
    }

    #[test]
    fn test_coyote_time() {
        // Verify coyote time is positive
        assert!(COYOTE_TIME_DURATION > 0.0);
    }
}
