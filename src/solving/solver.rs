use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use rayon::prelude::*;

fn read_lines(path: &str) -> io::Result<Vec<String>> {
    let file = File::open(Path::new(path))?;
    let reader = io::BufReader::new(file);
    Ok(reader.lines().collect::<Result<Vec<_>, _>>()?)
}

fn read_lines_first(paths: &[&str]) -> io::Result<(String, Vec<String>)> {
    for path in paths {
        if let Ok(lines) = read_lines(path) {
            return Ok(((*path).to_string(), lines));
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("none of these files exist: {}", paths.join(", ")),
    ))
}

#[derive(Clone)]
struct PackedWord {
    text: String,
    bytes: [u8; 5],
}

fn pack_word(raw: String) -> Option<PackedWord> {
    let word = raw.trim().to_ascii_lowercase();
    let bytes = word.as_bytes();
    if bytes.len() != 5 || !bytes.iter().all(|b| b.is_ascii_lowercase()) {
        return None;
    }
    let packed = [bytes[0], bytes[1], bytes[2], bytes[3], bytes[4]];

    Some(PackedWord {
        text: word,
        bytes: packed,
    })
}

#[allow(dead_code)]
pub struct Words {
    target_cnt: usize,
    target_path: String,
    pub target_words: Vec<String>,
    target_words_bytes: Vec<[u8; 5]>,
    possible_cnt: usize,
    possible_path: String,
    possible_words: Vec<PackedWord>,
}

impl Words {
    pub fn new() -> Self {
        let (target_path, target_raw) =
            read_lines_first(&["wordle_targets.txt"]).unwrap_or((String::new(), Vec::new()));
        let (possible_path, possible_raw) =
            read_lines_first(&["wordle_possibles.txt", "world_possibles.txt"])
                .unwrap_or((String::new(), Vec::new()));

        let target_packed: Vec<PackedWord> = target_raw.into_iter().filter_map(pack_word).collect();
        let possible_words: Vec<PackedWord> =
            possible_raw.into_iter().filter_map(pack_word).collect();

        let target_words: Vec<String> = target_packed.iter().map(|w| w.text.clone()).collect();
        let target_words_bytes: Vec<[u8; 5]> = target_packed.iter().map(|w| w.bytes).collect();

        let target_cnt = target_words.len();
        let possible_cnt = possible_words.len();

        Words {
            target_cnt,
            target_path,
            target_words,
            target_words_bytes,
            possible_cnt,
            possible_path,
            possible_words,
        }
    }
}

#[derive(Clone, Copy)]
pub enum Color {
    Grey,
    Yellow,
    Green,
}

impl From<u8> for Color {
    fn from(val: u8) -> Self {
        match val {
            2 => Color::Green,
            1 => Color::Yellow,
            _ => Color::Grey,
        }
    }
}

// [(char, Color); 5] — one entry per letter position
pub type Word = [(char, Color); 5];

pub struct WordleSolver {
    all_words: Words,
    curr_words: Vec<([u8; 5], [u8; 5])>,
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
        let mut guess = [b'a'; 5];
        let mut pattern = [0u8; 5];
        for i in 0..5 {
            guess[i] = word[i].0 as u8;
            pattern[i] = match word[i].1 {
                Color::Grey => 0,
                Color::Yellow => 1,
                Color::Green => 2,
            };
        }
        self.curr_words.push((guess, pattern));
    }

    /// Returns every target word still consistent with all guesses so far.
    pub fn possible_words(&self) -> Vec<String> {
        self.remaining_target_indices()
            .into_iter()
            .map(|idx| self.all_words.target_words[idx].clone())
            .collect()
    }

    /// Probability that `word` is the answer given current knowledge.
    /// Returns 1/N where N is the number of remaining possible words,
    /// or 0.0 if the word has already been ruled out.
    #[allow(dead_code)]
    pub fn probability(&self, word: &str) -> f32 {
        let remaining = self.remaining_target_indices();
        if remaining.is_empty() {
            return 0.0;
        }
        if remaining
            .iter()
            .any(|&idx| self.all_words.target_words[idx] == word)
        {
            1.0 / remaining.len() as f32
        } else {
            0.0
        }
    }

    /// Expected information gain (bits) from guessing `word` given a
    /// pre-computed list of remaining possible words.
    /// Uses a fixed [u32; 243] array (3^5 patterns) instead of a HashMap
    /// to avoid heap allocation in the hot path.
    fn get_expected_bits_with(&self, guess: [u8; 5], remaining: &[usize]) -> f32 {
        if remaining.is_empty() {
            return 0.0;
        }

        // 3^5 = 243 possible colour patterns, indexed in base-3
        let mut buckets = [0u32; 243];

        for &idx in remaining {
            let target = self.all_words.target_words_bytes[idx];
            let pattern = Self::score_bytes(guess, target);
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
        let remaining = self.remaining_target_indices();
        let bytes = word.as_bytes();
        if bytes.len() != 5 {
            return 0.0;
        }
        self.get_expected_bits_with(
            [bytes[0], bytes[1], bytes[2], bytes[3], bytes[4]],
            &remaining,
        )
    }

    /// Returns the best next guess (highest expected bits) from all target words.
    /// Computes `possible_words()` once and shares it across all candidates,
    /// then uses Rayon to evaluate candidates in parallel.
    pub fn best_guess(&self) -> Option<String> {
        let remaining = self.remaining_target_indices();
        if remaining.is_empty() {
            return None;
        }

        self.all_words
            .target_words_bytes
            .par_iter() // parallel iterator via Rayon
            .enumerate()
            .map(|(idx, &candidate)| (self.get_expected_bits_with(candidate, &remaining), idx))
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(_, idx)| self.all_words.target_words[idx].clone())
    }

    fn remaining_target_indices(&self) -> Vec<usize> {
        self.all_words
            .target_words_bytes
            .iter()
            .enumerate()
            .filter_map(|(idx, &candidate)| {
                let valid = self
                    .curr_words
                    .iter()
                    .all(|&(guess, pattern)| Self::score_bytes(guess, candidate) == pattern);
                if valid { Some(idx) } else { None }
            })
            .collect()
    }

    fn score_bytes(guess: [u8; 5], target: [u8; 5]) -> [u8; 5] {
        let mut pattern = [0u8; 5];
        let mut target_used = [false; 5];

        for i in 0..5 {
            if guess[i] == target[i] {
                pattern[i] = 2;
                target_used[i] = true;
            }
        }

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

    /// Scores a guess against a target, returning a 5-byte pattern:
    ///   2 = Green, 1 = Yellow, 0 = Grey
    pub fn score(guess: &str, target: &str) -> [u8; 5] {
        let gb = guess.as_bytes();
        let tb = target.as_bytes();
        if gb.len() != 5 || tb.len() != 5 {
            return [0u8; 5];
        }
        Self::score_bytes(
            [gb[0], gb[1], gb[2], gb[3], gb[4]],
            [tb[0], tb[1], tb[2], tb[3], tb[4]],
        )
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
