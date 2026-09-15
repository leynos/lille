//! Reading a shell script for the commands that will actually run.
//!
//! Split from `compiler_cache`, whose subject is the workflows as they
//! stand; this is the reading its resource-sampling assertion rests on.
//!
//! Deliberately bounded rather than a shell parser: it blanks what is
//! quoted, escaped or commented, then recognizes shell words. That is
//! enough to separate a command from text that merely spells one, which
//! is the whole question a sampling requirement turns on.
//!
//! Machinery only. The cases that drive it, including the shapes the
//! workflows deliberately do not contain, live in the contract module
//! `sampler_reading`, so neither file carries both the reading and the
//! question asked of it.

use std::fmt;
use std::iter::Peekable;
use std::str::Chars;

/// A command whose execution a contract requires.
///
/// Named rather than passed as text so the reading below cannot confuse
/// what it is looking for with the line it is looking in.
#[derive(Clone, Copy)]
pub struct Measure(pub &'static str);

impl fmt::Display for Measure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

/// One shell word, compared to a token whole rather than as letters.
#[derive(Clone, Copy)]
struct Word(&'static str);

/// A sequence of shell words that disables what it surrounds.
#[derive(Clone, Copy)]
struct Shape(&'static [&'static str]);

/// The words that open and close a conditional block.
const IF: Word = Word("if");
const FI: Word = Word("fi");

/// Shapes that disable a command while keeping its text.
///
/// `if false; then free -m; fi` satisfies a substring search while
/// sampling nothing, so the resource requirement in `compiler_cache`
/// would be met by a sampler that never ran. `free -m || true` runs but
/// discards its verdict.
///
/// Narrower than rejecting every compound line, deliberately. The
/// samplers here legitimately use pipes and command substitution, as in
/// `mem_used="$(free -m | awk ...)"`, so only the disabling forms are
/// refused rather than every line that is more than a bare command.
///
/// Written as word sequences rather than as text. The spacing between
/// an operator and its operand is the shell's to choose, so
/// `free -m ||    true` discards the verdict exactly as the one-space
/// spelling does while a substring search for `|| true` misses it.
const DISABLING_FORMS: [Shape; 2] = [Shape(&["||", "true"]), Shape(&["||", ":"])];

/// Shapes that open a block whose body never runs.
///
/// A guard is not confined to its own line. `if false` followed by
/// `free -m` on the next line and `fi` on the third disables the sampler
/// as thoroughly as the one-line form, and a line-by-line search sees an
/// undisabled `free -m` in the middle of it.
///
/// Word sequences again, so `if [ 1 -eq 0 ]` is recognized however its
/// words are spaced, and so the depth that tracks the guard counts the
/// words `if` and `fi` rather than their letters: `find` contains `fi`
/// and would otherwise close a guard from inside its own body.
const DISABLING_GUARDS: [Shape; 2] = [
    Shape(&["if", "false"]),
    Shape(&["if", "[", "1", "-eq", "0", "]"]),
];

/// Tracks whether the scanner is inside quotes or a command substitution.
///
/// Quoted text is not a command: `echo 'df -m'` prints three characters
/// and samples nothing. Command substitution is a command even inside
/// double quotes, which is how this repository's samplers are written:
/// `mem_used="$(free -m | awk '{print $3}')"`, so entering `$(` stacks
/// the quoting and leaving it restores what was there before.
#[derive(Default)]
struct Quoting {
    quoted: bool,
    closer: Option<char>,
    stack: Vec<(bool, Option<char>)>,
}

impl Quoting {
    /// Records a quote character, opening or closing a quoted run.
    const fn quote(&mut self, ch: char) {
        match self.closer {
            Some(open) if open == ch => {
                self.closer = None;
                self.quoted = false;
            }
            Some(_) => {}
            None => {
                self.closer = Some(ch);
                self.quoted = true;
            }
        }
    }

    /// Enters a command substitution, whose contents run.
    fn enter(&mut self) {
        self.stack.push((self.quoted, self.closer));
        self.quoted = false;
        self.closer = None;
    }

    /// Whether this character closes a command substitution, leaving it
    /// if it does.
    ///
    /// Only an unquoted `)` closes one. A `)` inside quotes is ordinary
    /// text, as in `printf '%s' ')'`, and treating it as the close
    /// restores the quoting that surrounded the substitution over the
    /// rest of the line, blanking commands that will in fact run.
    fn closes_substitution(&mut self, ch: char) -> bool {
        ch == ')' && !self.quoted && self.leave()
    }

    /// Leaves a command substitution, restoring the quoting around it.
    fn leave(&mut self) -> bool {
        match self.stack.pop() {
            Some((quoted, closer)) => {
                self.quoted = quoted;
                self.closer = closer;
                true
            }
            None => false,
        }
    }

    /// Whether `$(` here opens a substitution.
    ///
    /// Single quotes suppress it; double quotes do not, which is the
    /// case the samplers rely on.
    fn allows_substitution(&self) -> bool {
        self.closer != Some('\'')
    }

    /// Whether a backslash here makes the next character literal.
    ///
    /// Single quotes are the one context where a backslash is an
    /// ordinary character. Everywhere else it escapes what follows, so
    /// `echo "\$(free -m)"` prints the text `$(free -m)` and samples
    /// nothing, though the unescaped spelling beside it does sample.
    const fn escapes(&self) -> bool {
        !matches!(self.closer, Some('\''))
    }

    /// Returns what one character, with the rest of the line, masks to.
    fn consume(&mut self, ch: char, rest: &mut Peekable<Chars<'_>>) -> String {
        if ch == '\\' && self.escapes() {
            // The backslash and what it escapes are both literal text,
            // so neither can open a substitution nor form an operator.
            // Blanking the escaped character as well can only hide a
            // command from the search, never invent one.
            return " ".repeat(1 + usize::from(rest.next().is_some()));
        }
        let opens = ch == '$' && rest.peek() == Some(&'(') && self.allows_substitution();
        if opens {
            rest.next();
            self.enter();
        }
        self.render(ch, opens)
    }

    /// Returns what one character contributes to the masked line.
    ///
    /// Spaces stand in for everything that will not be run, so the
    /// result lines up with the original and can be searched directly.
    /// A quote character is consumed here rather than by the caller,
    /// which is what keeps the scanning loop to one branch.
    fn render(&mut self, ch: char, opened: bool) -> String {
        if opened {
            return "  ".to_owned();
        }
        if self.closes_substitution(ch) {
            return " ".to_owned();
        }
        if ch == '\'' || ch == '"' {
            self.quote(ch);
            return " ".to_owned();
        }
        if self.quoted {
            return " ".to_owned();
        }
        ch.to_string()
    }
}

/// Returns the line with quoted and escaped text blanked out.
///
/// Characters are replaced by spaces rather than removed, so what comes
/// back can be searched directly for a command's text.
fn mask(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut state = Quoting::default();
    while let Some(ch) = chars.next() {
        out.push_str(&state.consume(ch, &mut chars));
    }
    out
}

/// One line of a `run` script, with what will not run blanked out.
struct ExecutableLine(String);

impl ExecutableLine {
    /// Reads one raw line of a script.
    ///
    /// The comment is cut by position, which is safe because the mask
    /// has already blanked every quoted character: a `#` surviving into
    /// its output is one the shell would read as starting a comment.
    fn read(line: &str) -> Self {
        let masked = mask(line);
        let executable = masked
            .split_once('#')
            .map_or(masked.as_str(), |(before, _)| before);
        Self(executable.to_owned())
    }

    /// Whether the measure's text survives into what will be executed.
    fn names(&self, measure: Measure) -> bool {
        self.0.contains(measure.0)
    }

    /// Splits the line into shell words and operators.
    fn tokens(&self) -> Tokens {
        Tokens::read(&self.0)
    }
}

/// Characters the shell reads as operators rather than as word text.
///
/// Split off as tokens of their own so `false;` and `false` are the
/// same word, with a run of one character kept together so `||` stays
/// one token and is not two `|`.
const OPERATOR_CHARS: [char; 3] = [';', '|', '&'];

/// Returns a run of one operator character as a single token.
fn operator_run(ch: char, rest: &mut Peekable<Chars<'_>>) -> String {
    let mut operator = String::from(ch);
    while rest.peek() == Some(&ch) {
        operator.push(ch);
        rest.next();
    }
    operator
}

/// Accumulates shell words and operators as characters arrive.
#[derive(Default)]
struct TokenReader {
    tokens: Vec<String>,
    word: String,
}

impl TokenReader {
    /// Takes one character, and whatever an operator run needs after it.
    fn take(&mut self, ch: char, rest: &mut Peekable<Chars<'_>>) {
        if !ch.is_whitespace() && !OPERATOR_CHARS.contains(&ch) {
            self.word.push(ch);
            return;
        }
        self.flush();
        if OPERATOR_CHARS.contains(&ch) {
            self.tokens.push(operator_run(ch, rest));
        }
    }

    /// Ends the word in progress, when there is one.
    fn flush(&mut self) {
        if !self.word.is_empty() {
            self.tokens.push(std::mem::take(&mut self.word));
        }
    }

    /// Ends the last word and returns what was read.
    fn finish(mut self) -> Tokens {
        self.flush();
        Tokens(self.tokens)
    }
}

/// The shell words and operators of one executable line.
///
/// Bounded recognition rather than a shell parser, and enough for what
/// this reading asks: the mask has already blanked everything quoted,
/// so what remains is words separated by whitespace and runs of
/// operator characters. That is what tells the word `fi` from the word
/// `find`, and `||    true` from an argument that merely reads that way.
struct Tokens(Vec<String>);

impl Tokens {
    /// Reads the words and operators out of one masked line.
    fn read(text: &str) -> Self {
        let mut reader = TokenReader::default();
        let mut chars = text.chars().peekable();
        while let Some(ch) = chars.next() {
            reader.take(ch, &mut chars);
        }
        reader.finish()
    }

    /// Returns how many tokens are exactly one word.
    fn count(&self, word: Word) -> usize {
        self.0
            .iter()
            .filter(|found| found.as_str() == word.0)
            .count()
    }

    /// Returns whether the tokens carry a shape adjacently and in order.
    fn carries(&self, shape: Shape) -> bool {
        self.0.windows(shape.0.len()).any(|window| {
            window
                .iter()
                .zip(shape.0)
                .all(|(token, want)| token.as_str() == *want)
        })
    }

    /// Returns whether the line throws away the verdict of what it ran.
    fn discards_verdict(&self) -> bool {
        DISABLING_FORMS.iter().any(|form| self.carries(*form))
    }

    /// Returns whether the line opens a guard whose body never runs.
    fn opens_disabling_guard(&self) -> bool {
        DISABLING_GUARDS.iter().any(|guard| self.carries(*guard))
    }
}

/// Returns whether a job samples one measure in a form that runs.
///
/// Three things have to hold: the measure appears as text that will be
/// executed rather than printed or commented out, the line does not
/// discard its verdict, and the line is not inside a guard whose body
/// never runs.
///
/// # Examples
///
/// ```ignore
/// assert!(samples(&job, Measure("free -m")));  // mem="$(free -m | awk ...)"
/// assert!(!samples(&job, Measure("df -m")));   // if false; then df -m; fi
/// assert!(!samples(&job, Measure("df -m")));   // echo 'df -m'
/// ```
pub fn samples(job: &crate::workflow_model::Job, measure: Measure) -> bool {
    job.steps
        .iter()
        .any(|step| step_samples(&step.run, measure))
}

/// Returns whether one `run` script samples a measure in a form that runs.
pub fn step_samples(run: &str, measure: Measure) -> bool {
    let mut guard_depth: usize = 0;
    for raw in run.lines() {
        let line = ExecutableLine::read(raw);
        let tokens = line.tokens();
        let closes = tokens.count(FI);
        let opens = tokens.count(IF);
        if guard_depth > 0 {
            guard_depth = guard_depth.saturating_add(opens).saturating_sub(closes);
            continue;
        }
        if tokens.opens_disabling_guard() {
            guard_depth = opens.saturating_sub(closes);
            continue;
        }
        if line.names(measure) && !tokens.discards_verdict() {
            return true;
        }
    }
    false
}
