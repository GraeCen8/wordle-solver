mod solving;

fn main() {
    solving::solver::example()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solving::solver::Word;
    use crate::solving::solver::WordleSolver;

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

            let remaining = solver.possible_words();
        }
        println!("Failed to solve '{}'");
        None
    }
}
