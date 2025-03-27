#![warn(clippy::all, clippy::pedantic)]

#[cfg(test)]
mod tests {
    use crate::config::CONFIG;
    use crate::config::menu::{KerningPair, TitleConfig};
    use crate::menu::title::{TITLE_LETTERS, get_title_protection_zone};
    use ratatui::layout::Rect;

    #[test]
    fn test_title_letters() {
        // Verify we have all letters for "FALLINGBLOCKS"
        assert_eq!(TITLE_LETTERS.len(), 13);

        // Check all letters have height lines
        for letter in TITLE_LETTERS {
            let lines: Vec<&str> = letter.lines().collect();
            assert_eq!(lines.len(), 5); // Title height
        }
    }

    #[test]
    fn test_kerning_calculation() {
        let kerning_pairs = vec![
            KerningPair {
                letter_index: 0,
                adjustment: -5,
            },
            KerningPair {
                letter_index: 1,
                adjustment: -3,
            },
            KerningPair {
                letter_index: 2,
                adjustment: -2,
            },
        ];

        let total_adjustment = crate::config::menu::get_total_kerning_adjustment(&kerning_pairs);

        // Sum of all negative adjustments converted to positive
        assert_eq!(total_adjustment, 10);

        // Test with positive values (should be 0)
        let positive_pairs = vec![
            KerningPair {
                letter_index: 0,
                adjustment: 5,
            },
            KerningPair {
                letter_index: 1,
                adjustment: 3,
            },
        ];

        let adjustment = crate::config::menu::get_total_kerning_adjustment(&positive_pairs);
        assert_eq!(adjustment, 0);
    }

    #[test]
    fn test_get_title_protection_zone() {
        // Override config values for the test
        {
            let mut config = CONFIG.write().unwrap();
            config.menu.title.title_height = 6;
            config.menu.title.protection_margin = 4;

            // Modify some kerning values to test width calculations
            if let Some(kerning) = config
                .menu
                .title
                .kerning_adjustments
                .iter_mut()
                .find(|p| p.letter_index == 0)
            {
                kerning.adjustment = -10;
            }
        }

        // Create an area for testing
        let area = Rect::new(0, 0, 100, 50);

        // Get protection zone
        let protection_zone = get_title_protection_zone(area);

        // Check dimensions
        assert_eq!(protection_zone.x, 0);
        assert_eq!(protection_zone.y, 0);
        assert_eq!(protection_zone.width, 100); // Full width

        // Height should be title height + protection margin
        // Since the exact height depends on the centered calculation, we just check it's reasonable
        assert!(
            protection_zone.height > 10,
            "Protection zone height should include title height + margin"
        );
    }

    #[test]
    fn test_title_config_custom_adjustments() {
        let mut title_config = TitleConfig::default();

        // Set custom values
        title_config.title_height = 7;
        title_config.protection_margin = 5;

        // Custom kerning adjustments
        title_config.kerning_adjustments = vec![
            KerningPair {
                letter_index: 0,
                adjustment: -10,
            }, // F->A
            KerningPair {
                letter_index: 1,
                adjustment: -7,
            }, // A->L
            KerningPair {
                letter_index: 2,
                adjustment: -4,
            }, // L->L
        ];

        // Verify custom values
        assert_eq!(title_config.title_height, 7);
        assert_eq!(title_config.protection_margin, 5);
        assert_eq!(title_config.kerning_adjustments.len(), 3);

        // Check specific kerning
        assert_eq!(title_config.kerning_adjustments[0].letter_index, 0);
        assert_eq!(title_config.kerning_adjustments[0].adjustment, -10);

        // Calculate total kerning
        let total_kerning =
            crate::config::menu::get_total_kerning_adjustment(&title_config.kerning_adjustments);
        assert_eq!(total_kerning, 21); // Sum of absolute values of negative adjustments
    }

    #[test]
    fn test_title_character_limits() {
        // Verify all letters have defined pixel art
        for (i, letter) in "FALLINGBLOCKS".chars().enumerate() {
            assert!(
                i < TITLE_LETTERS.len(),
                "Missing ASCII art for letter {}",
                letter
            );
        }

        // Test default kerning adjustments for all letter pairs
        let title_config = TitleConfig::default();

        // Verify all the letter pairs have kerning defined
        for i in 0..("FALLINGBLOCKS".len() - 1) {
            let has_kerning = title_config
                .kerning_adjustments
                .iter()
                .any(|k| k.letter_index == i);

            assert!(
                has_kerning,
                "Letter pair at index {} is missing kerning adjustment",
                i
            );
        }
    }
}
