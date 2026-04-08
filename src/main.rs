mod solving;

fn main() {
    solving::solver::example()
}

#[cfg(test)]
mod tests {
    use crate::solving::solver::Word;
    use crate::solving::solver::WordleSolver;
    use crate::solving::solver::Words;

    use rayon::prelude::*;

    //solves a worlde for a given target word returning the num or guesses or None if failed in 6
    //guesses
    fn solve(target: &str) -> Option<usize> {
        let mut solver = WordleSolver::new();

        for guess_num in 1..=6 {
            let guess = solver.best_guess().expect("no guess returned");

            //check if it is solved
            if guess == target {
                return Some(guess_num);
            }

            let pattern = WordleSolver::score(&guess, target);
            let chars: Vec<char> = guess.chars().collect();

            let word: Word = [
                (chars[0], pattern[0].into()),
                (chars[1], pattern[1].into()),
                (chars[2], pattern[2].into()),
                (chars[3], pattern[3].into()),
                (chars[4], pattern[4].into()),
            ];
            solver.add_guess(word);
        }
        println!("Failed to solve '{}'", target);
        None
    }

    #[test]
    fn solve_all_words() {
        let all_words = Words::new().target_words;

        let answers: Vec<Option<usize>> = all_words
            .par_iter()
            .map(|test_word| solve(test_word))
            .collect();

        let succeded_words: Vec<usize> = answers.clone().into_iter().flatten().collect();
        let none_cnt = answers.clone().iter().filter(|a| a.is_none()).count() as i32;
        let avg_moves: f32 =
            succeded_words.iter().sum::<usize>() as f32 / succeded_words.len() as f32;

        println!("the avg moves of all words is: {}", avg_moves);
        print!("the amount of words that failed is {}", none_cnt);
        assert_eq!(none_cnt, 0 as i32)
    }
}
