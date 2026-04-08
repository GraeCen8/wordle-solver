use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use rayon::prelude::*;

fn read_lines(path: &str) -> io::Result<Vec<String>> {
    let file = File::open(Path::new(path))?;
    let reader = io::BufReader::new(file);
    Ok(reader.lines().collect::<Result<Vec<_>, _>>()?)
}

#[allow(dead_code)]
struct Words {
    target_cnt: usize,
    target_path: String,
    target_words: Vec<String>,
    possible_cnt: usize,
    possible_path: String,
    possible_words: Vec<String>,
}

impl Words {
    pub fn new() -> Self {
        let target_path: String = "wordle_targets.txt".into();
        let possible_path: String = "wordle_possibles.txt".into();

        let target_words: Vec<String> = read_lines(&target_path)
            .unwrap_or_default()
            .into_iter()
            .map(|w| w.trim().to_ascii_lowercase())
            .filter(|w| w.len() == 5 && w.chars().all(|c| c.is_ascii_lowercase()))
            .collect();

        let possible_words: Vec<String> = read_lines(&possible_path)
            .unwrap_or_default()
            .into_iter()
            .map(|w| w.trim().to_ascii_lowercase())
            .filter(|w| w.len() == 5 && w.chars().all(|c| c.is_ascii_lowercase()))
            .collect();

        let target_cnt = target_words.len();
        let possible_cnt = possible_words.len();

        Words {
            target_cnt,
            target_path,
            target_words,
            possible_cnt,
            possible_path,
            possible_words,
        }
    }
}

pub enum Color {
    Grey,
    Yellow,
    Green,
}

// [(char, Color); 5] — one entry per letter position
pub type Word = [(char, Color); 5];

pub struct WordleSolver {
    all_words: Words,
    curr_words: Vec<Word>,
}

impl WordleSolver {
    pub fn new() -> Self {
        WordleSolver {
            all_words: Words::new(),
            curr_words: Vec::new(),
        }
    }

    /// Add a guess and its resulting colours to the solver state.
    pub fn add_guess(&mut self, word: Word) {
        self.curr_words.push(word);
    }

    /// Returns every target word still consistent with all guesses so far.
    pub fn possible_words(&self) -> Vec<String> {
        let all_words = &self.all_words.target_words;

        all_words
            .iter()
            .filter(|candidate| {
                self.curr_words.iter().all(|guessed_word| {
                    guessed_word
                        .iter()
                        .enumerate()
                        .all(|(i, (guessed_char, color))| match color {
                            // Grey: char must not appear anywhere in the candidate.
                            Color::Grey => !candidate.contains(*guessed_char),

                            // Green: candidate must have this exact char at this exact position
                            Color::Green => candidate.chars().nth(i) == Some(*guessed_char),

                            // Yellow: char exists in candidate but NOT at this position
                            Color::Yellow => {
                                candidate.contains(*guessed_char)
                                    && candidate.chars().nth(i) != Some(*guessed_char)
                            }
                        })
                })
            })
            .cloned()
            .collect()
    }

    /// Probability that `word` is the answer given current knowledge.
    /// Returns 1/N where N is the number of remaining possible words,
    /// or 0.0 if the word has already been ruled out.
    pub fn probability(&self, word: &str) -> f32 {
        let words = self.possible_words();
        if words.is_empty() {
            return 0.0;
        }
        if words.iter().any(|candidate| candidate == word) {
            1.0 / words.len() as f32
        } else {
            0.0
        }
    }

    /// Expected information gain (bits) from guessing `word` given a
    /// pre-computed list of remaining possible words.
    /// Uses a fixed [u32; 243] array (3^5 patterns) instead of a HashMap
    /// to avoid heap allocation in the hot path.
    fn get_expected_bits_with(&self, word: &str, remaining: &[String]) -> f32 {
        if remaining.is_empty() {
            return 0.0;
        }

        // 3^5 = 243 possible colour patterns, indexed in base-3
        let mut buckets = [0u32; 243];

        for target in remaining {
            let pattern = Self::score(word, target);
            let idx = pattern.iter().fold(0usize, |acc, &p| acc * 3 + p as usize);
            buckets[idx] += 1;
        }

        let total = remaining.len() as f32;
        buckets
            .iter()
            .filter(|&&c| c > 0)
            .map(|&count| {
                let p = count as f32 / total;
                -p * p.log2()
            })
            .sum()
    }

    /// Public wrapper — computes remaining words then delegates.
    pub fn get_expected_bits(&self, word: &str) -> f32 {
        let remaining = self.possible_words();
        self.get_expected_bits_with(word, &remaining)
    }

    /// Returns the best next guess (highest expected bits) from all target words.
    /// Computes `possible_words()` once and shares it across all candidates,
    /// then uses Rayon to evaluate candidates in parallel.
    pub fn best_guess(&self) -> Option<String> {
        let remaining = self.possible_words(); // computed once, shared across all candidates
        let candidates = &self.all_words.target_words;

        candidates
            .par_iter() // parallel iterator via Rayon
            .max_by(|a, b| {
                self.get_expected_bits_with(a, &remaining)
                    .partial_cmp(&self.get_expected_bits_with(b, &remaining))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    /// Scores a guess against a target, returning a 5-byte pattern:
    ///   2 = Green, 1 = Yellow, 0 = Grey
    fn score(guess: &str, target: &str) -> [u8; 5] {
        let guess: Vec<char> = guess.chars().collect();
        let target: Vec<char> = target.chars().collect();
        let mut pattern = [0u8; 5];
        let mut target_used = [false; 5];

        // First pass: greens
        for i in 0..5 {
            if guess[i] == target[i] {
                pattern[i] = 2;
                target_used[i] = true;
            }
        }

        // Second pass: yellows
        for i in 0..5 {
            if pattern[i] == 2 {
                continue;
            }
            for j in 0..5 {
                if !target_used[j] && guess[i] == target[j] {
                    pattern[i] = 1;
                    target_used[j] = true;
                    break;
                }
            }
        }

        pattern
    }
}

#[allow(dead_code)]
pub fn example() {
    let mut solver = WordleSolver::new();

    println!("Remaining words: {}", solver.possible_words().len());

    if let Some(guess) = solver.best_guess() {
        println!("Best opening guess: {}", guess);
        println!("Expected bits: {:.4}", solver.get_expected_bits(&guess));
    }

    solver.add_guess([
        ('c', Color::Grey),
        ('r', Color::Green),
        ('a', Color::Yellow),
        ('n', Color::Grey),
        ('e', Color::Grey),
    ]);

    let remaining = solver.possible_words();
    println!("\nAfter guess, remaining words: {}", remaining.len());
    for w in &remaining {
        println!("  {}", w);
    }

    if let Some(next) = solver.best_guess() {
        println!("\nBest next guess: {}", next);
    }
}

