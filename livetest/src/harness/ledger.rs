//! Operator-local run-ledger for resources the sweep can't identify by an
//! in-account marker -- chiefly DIDs, which have no safe free-text field and
//! whose numbers can be reassigned to other customers after cancellation.
//!
//! The moment such a resource is created the harness appends an entry here, and
//! removes it on successful teardown. On startup the sweep reconciles: any
//! still-listed entry for the current account is looked up in-account and, if
//! it still exists, cleaned up. The ledger is a convenience, never authoritative
//! over account state; if the two disagree, the account wins.
//!
//! The file may contain DIDs and usernames, so it is git-ignored and lives
//! outside the repo tree by default. It is keyed by a hash of the account so a
//! shared ledger path never reconciles one account's entries against another.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// One tracked resource. `kind` names the resource type (e.g. `"did"`),
/// `id` is the identifier needed to look it up and delete it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entry {
    /// Opaque account key (a non-reversible tag of the API username) so entries
    /// for different accounts sharing a ledger file never cross-reconcile.
    pub account: String,
    pub kind: String,
    pub id: String,
    /// The run token that created it, for attribution in logs.
    pub run: String,
}

/// Append-only-ish JSONL ledger. Removal rewrites the file without the entry.
pub struct Ledger {
    path: PathBuf,
    account: String,
}

impl Ledger {
    /// Open (creating on first write) a ledger at `path`, scoped to `username`.
    pub fn open(path: PathBuf, username: &str) -> Self {
        Ledger {
            path,
            account: account_key(username),
        }
    }

    /// Append an entry for a freshly created resource.
    pub fn append(&self, kind: &str, id: &str, run: &str) -> Result<()> {
        let entry = Entry {
            account: self.account.clone(),
            kind: kind.to_string(),
            id: id.to_string(),
            run: run.to_string(),
        };

        let line = serde_json::to_string(&entry).context("serializing ledger entry")?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("opening ledger {}", self.path.display()))?;
        writeln!(file, "{line}").context("appending to ledger")?;
        Ok(())
    }

    /// Remove the first entry matching `kind`+`id` for this account.
    pub fn remove(&self, kind: &str, id: &str) -> Result<()> {
        let mut entries = self.read_all()?;
        let before = entries.len();
        let mut removed = false;
        entries.retain(|e| {
            if !removed && e.account == self.account && e.kind == kind && e.id == id {
                removed = true;
                false
            } else {
                true
            }
        });

        if entries.len() != before {
            self.rewrite(&entries)?;
        }

        Ok(())
    }

    /// Entries for this account only, for reconciliation at startup.
    pub fn entries_for_account(&self) -> Result<Vec<Entry>> {
        Ok(self
            .read_all()?
            .into_iter()
            .filter(|e| e.account == self.account)
            .collect())
    }

    fn read_all(&self) -> Result<Vec<Entry>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.path)
            .with_context(|| format!("reading ledger {}", self.path.display()))?;
        let mut out = Vec::new();
        for line in BufReader::new(file).lines() {
            let line = line.context("reading ledger line")?;
            if line.trim().is_empty() {
                continue;
            }
            // A malformed line is skipped rather than aborting reconciliation --
            // the account remains the source of truth regardless.
            if let Ok(entry) = serde_json::from_str::<Entry>(&line) {
                out.push(entry);
            }
        }

        Ok(out)
    }

    fn rewrite(&self, entries: &[Entry]) -> Result<()> {
        let mut buf = String::new();
        for e in entries {
            buf.push_str(&serde_json::to_string(e).context("serializing ledger entry")?);
            buf.push('\n');
        }

        let mut file = File::create(&self.path)
            .with_context(|| format!("rewriting ledger {}", self.path.display()))?;
        file.write_all(buf.as_bytes()).context("writing ledger")?;
        Ok(())
    }
}

/// A stable, non-reversible key for an account username. Not cryptographic --
/// just enough to segregate ledger entries and avoid writing the raw username.
fn account_key(username: &str) -> String {
    // FNV-1a 64-bit: deterministic, dependency-free, adequate for segregation.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in username.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    format!("{hash:016x}")
}

/// Convenience for `main` to resolve the effective ledger path for display.
pub fn describe_path(path: &Path) -> String {
    path.display().to_string()
}
