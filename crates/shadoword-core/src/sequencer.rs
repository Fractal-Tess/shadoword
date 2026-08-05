use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequencerError {
    Terminal(String),
    AlreadyTerminal,
    StaleSequence(u64),
    DuplicateSequence(u64),
    BufferFull,
}

impl fmt::Display for SequencerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Terminal(message) => write!(formatter, "ordered sequence terminated: {message}"),
            Self::AlreadyTerminal => formatter.write_str("ordered sequence is terminal"),
            Self::StaleSequence(sequence) => {
                write!(formatter, "sequence {sequence} was already emitted")
            }
            Self::DuplicateSequence(sequence) => {
                write!(formatter, "sequence {sequence} completed more than once")
            }
            Self::BufferFull => formatter.write_str("ordered completion buffer is full"),
        }
    }
}

impl std::error::Error for SequencerError {}

#[derive(Debug)]
pub struct OrderedCompletion<T> {
    next_sequence: u64,
    max_buffered: usize,
    buffered: BTreeMap<u64, T>,
    terminal: Option<String>,
}

impl<T> OrderedCompletion<T> {
    pub fn new(first_sequence: u64, max_buffered: usize) -> Self {
        assert!(
            max_buffered > 0,
            "ordered completion buffer must be non-zero"
        );
        Self {
            next_sequence: first_sequence,
            max_buffered,
            buffered: BTreeMap::new(),
            terminal: None,
        }
    }

    pub fn next_sequence(&self) -> u64 {
        self.next_sequence
    }

    pub fn buffered_len(&self) -> usize {
        self.buffered.len()
    }

    pub fn is_terminal(&self) -> bool {
        self.terminal.is_some()
    }

    pub fn complete(&mut self, sequence: u64, value: T) -> Result<Vec<T>, SequencerError> {
        if let Some(message) = &self.terminal {
            return Err(SequencerError::Terminal(message.clone()));
        }
        if sequence < self.next_sequence {
            return Err(SequencerError::StaleSequence(sequence));
        }
        if self.buffered.contains_key(&sequence) {
            return Err(SequencerError::DuplicateSequence(sequence));
        }
        if sequence != self.next_sequence && self.buffered.len() >= self.max_buffered {
            return Err(SequencerError::BufferFull);
        }

        self.buffered.insert(sequence, value);
        let mut ready = Vec::new();
        while let Some(value) = self.buffered.remove(&self.next_sequence) {
            ready.push(value);
            self.next_sequence += 1;
        }
        Ok(ready)
    }

    pub fn fail(
        &mut self,
        sequence: u64,
        message: impl Into<String>,
    ) -> Result<(), SequencerError> {
        if self.terminal.is_some() {
            return Err(SequencerError::AlreadyTerminal);
        }
        if sequence < self.next_sequence {
            return Err(SequencerError::StaleSequence(sequence));
        }
        if self.buffered.contains_key(&sequence) {
            return Err(SequencerError::DuplicateSequence(sequence));
        }
        self.terminal = Some(format!("sequence {sequence}: {}", message.into()));
        self.buffered.clear();
        Ok(())
    }
}
