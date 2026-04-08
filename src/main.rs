mod solving;

fn main() {
    solving::solver::example()
}

#[cfg(test)]
mod tests {
    use crate::solving::solver::{Color, Word, WordleSolver, Words};
    use std::collections::HashSet;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn feedback_from_guess_and_target(guess: &str, target: &str) -> Word {
        let pattern = WordleSolver::score(guess, target);
        let chars: Vec<char> = guess.chars().collect();
        [
            (chars[0], pattern[0].into()),
            (chars[1], pattern[1].into()),
            (chars[2], pattern[2].into()),
            (chars[3], pattern[3].into()),
            (chars[4], pattern[4].into()),
        ]
    }

    fn solve_with_repeat_guard(target: &str, max_guesses: usize) -> Option<usize> {
        let mut solver = WordleSolver::new();
        let mut seen: HashSet<String> = HashSet::new();

        for guess_num in 1..=max_guesses {
            let mut guess = solver.best_guess().expect("no guess returned");
            if seen.contains(&guess) {
                let fallback = solver
                    .possible_words()
                    .into_iter()
                    .find(|w| !seen.contains(w))
                    .expect("no unseen fallback candidate");
                guess = fallback;
            }

            seen.insert(guess.clone());
            if guess == target {
                return Some(guess_num);
            }

            let feedback = feedback_from_guess_and_target(&guess, target);
            solver.add_guess(feedback);
        }

        None
    }

    fn random_target_word() -> String {
        let words = Words::new().target_words;
        let len = words.len();
        assert!(len > 0, "target word list is empty");

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock error")
            .as_nanos() as u64;
        let mut x = nanos ^ 0x9E37_79B9_7F4A_7C15;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        words[(x as usize) % len].clone()
    }

    #[test]
    fn score_handles_repeated_letters() {
        assert_eq!(WordleSolver::score("allee", "apple"), [2, 1, 0, 0, 2]);
        assert_eq!(WordleSolver::score("sissy", "missy"), [0, 2, 2, 2, 2]);
    }

    #[test]
    fn best_guess_is_valid_and_has_bits() {
        let solver = WordleSolver::new();
        let guess = solver.best_guess().expect("no guess returned");

        assert_eq!(guess.len(), 5);
        assert!(guess.chars().all(|c| c.is_ascii_lowercase()));
        assert!(solver.get_expected_bits(&guess) >= 0.0);
    }

    #[test]
    fn adding_feedback_never_increases_candidate_count() {
        let mut solver = WordleSolver::new();
        let before = solver.possible_words().len();

        let feedback: Word = feedback_from_guess_and_target("crane", "slate");
        solver.add_guess(feedback);
        let after = solver.possible_words().len();

        assert!(after <= before);
    }

    #[test]
    fn all_green_feedback_leaves_exact_target() {
        let mut solver = WordleSolver::new();
        let target = "cigar";

        let feedback: Word = [
            ('c', Color::Green),
            ('i', Color::Green),
            ('g', Color::Green),
            ('a', Color::Green),
            ('r', Color::Green),
        ];
        solver.add_guess(feedback);
        let remaining = solver.possible_words();

        assert_eq!(remaining, vec![target.to_string()]);
    }

    #[test]
    fn solves_random_target_1() {
        let target = random_target_word();
        let solved_in = solve_with_repeat_guard(&target, 6);
        assert!(
            solved_in.is_some(),
            "failed to solve random target '{}' within 6 guesses",
            target
        );
    }

    #[test]
    fn solves_random_target_2() {
        let target = random_target_word();
        let solved_in = solve_with_repeat_guard(&target, 6);
        assert!(
            solved_in.is_some(),
            "failed to solve random target '{}' within 6 guesses",
            target
        );
    }

    #[test]
    fn solves_random_target_3() {
        let target = random_target_word();
        let solved_in = solve_with_repeat_guard(&target, 6);
        assert!(
            solved_in.is_some(),
            "failed to solve random target '{}' within 6 guesses",
            target
        );
    }
}
