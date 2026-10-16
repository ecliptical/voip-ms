//! HTML → `tools/api-responses.json` extractor.
//!
//! Parses the saved VoIP.ms API documentation HTML (which is gated by
//! Cloudflare and requires a logged-in browser to obtain), finds every
//! method's `Output` example block, decodes the PHP `print_r` notation
//! into a typed shape tree, and writes a machine-readable summary that
//! `cargo xtask gen` consumes to emit response structs.
//!
//! The HTML itself is never committed — only this extract is.
//!
//! Run from the repository root:
//!     cargo xtask extract-responses path/to/api-doc.html

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::wsdl;

/// Top-level JSON document written to `tools/api-responses.json`.
#[derive(Serialize, Deserialize)]
pub struct Document {
    pub version: u32,
    pub methods: serde_json::Map<String, JsonValue>,
    /// Per-method parameter descriptions mined from each method's
    /// `Parameters` cell in the HTML docs. Keyed by wire method name,
    /// then by wire parameter name. Optional so older extracts (and the
    /// override flow, which only reads `methods`) keep parsing.
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub param_docs: serde_json::Map<String, JsonValue>,
    /// Per-method notes mined from the paragraph that follows a method's
    /// parameter table, keyed by wire method name. A note describes the
    /// parameters as a group, so no parameter carries it.
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub param_notes: serde_json::Map<String, JsonValue>,
    /// Per-method one-line descriptions mined from each method's
    /// description row in the docs. Keyed by wire method name. Most methods
    /// carry one (~218 of 222); the rest are absent. Optional for
    /// forward/backward compatibility with extracts that predate this field.
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub method_docs: serde_json::Map<String, JsonValue>,
}

/// Inferred scalar primitive.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ScalarTy {
    #[serde(rename = "integer")]
    Integer,
    #[serde(rename = "decimal")]
    Decimal,
    #[serde(rename = "bool_yn")]
    BoolYn,
    #[serde(rename = "bool_01")]
    Bool01,
    #[serde(rename = "date")]
    Date,
    #[serde(rename = "datetime")]
    DateTime,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "empty")]
    Empty,
}

impl ScalarTy {
    fn as_str(self) -> &'static str {
        match self {
            ScalarTy::Integer => "integer",
            ScalarTy::Decimal => "decimal",
            ScalarTy::BoolYn => "bool_yn",
            ScalarTy::Bool01 => "bool_01",
            ScalarTy::Date => "date",
            ScalarTy::DateTime => "datetime",
            ScalarTy::String => "string",
            ScalarTy::Empty => "string",
        }
    }
}

/// Shape tree node.
#[derive(Debug, Clone)]
pub enum Shape {
    Scalar {
        ty: ScalarTy,
        sample: String,
    },
    Object(Vec<(String, Shape)>),
    /// A `[0] => …, [1] => …` array. The inner shape is the merged
    /// element template.
    List(Box<Shape>),
    /// A string-keyed map whose keys are data, not schema (a `code =>
    /// description` catalog like `getLNPListStatus`'s `list_status`). The inner
    /// shape is the value template; keys are always strings. The `print_r`
    /// extractor never infers this -- it comes only from a full shape in the
    /// overrides file, where an object with dynamic keys would otherwise be
    /// forced into a fixed-field struct.
    Map(Box<Shape>),
}

/// JSON-equivalent of `Shape` (de)serialized to/from `api-responses.json`.
/// The on-disk representation is tagged on `kind`.
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum ShapeRepr {
    Scalar {
        #[serde(rename = "type")]
        ty: ScalarTy,
        #[serde(default)]
        sample: String,
    },
    Object {
        fields: Vec<ShapeField>,
    },
    List {
        element: Box<ShapeRepr>,
    },
    Map {
        value: Box<ShapeRepr>,
    },
}

#[derive(Serialize, Deserialize)]
struct ShapeField {
    name: String,
    shape: ShapeRepr,
}

impl Shape {
    pub fn from_json(value: &JsonValue) -> Result<Self, String> {
        let repr: ShapeRepr =
            serde_json::from_value(value.clone()).map_err(|e| format!("decode shape: {e}"))?;
        Ok(Self::from_repr(repr))
    }

    fn from_repr(repr: ShapeRepr) -> Self {
        match repr {
            ShapeRepr::Scalar { ty, sample } => Shape::Scalar { ty, sample },
            ShapeRepr::Object { fields } => Shape::Object(
                fields
                    .into_iter()
                    .map(|f| (f.name, Self::from_repr(f.shape)))
                    .collect(),
            ),
            ShapeRepr::List { element } => Shape::List(Box::new(Self::from_repr(*element))),
            ShapeRepr::Map { value } => Shape::Map(Box::new(Self::from_repr(*value))),
        }
    }
}

/// Top-level JSON document written to `tools/api-statuses.json`.
#[derive(Serialize, Deserialize)]
pub struct StatusDocument {
    pub version: u32,
    /// Documented `status` values, in source order. Each entry is the
    /// verbatim wire string and its human-readable description.
    pub statuses: Vec<StatusEntry>,
}

/// One row of the global "Error Codes" table.
#[derive(Serialize, Deserialize)]
pub struct StatusEntry {
    /// Verbatim wire `status` string (e.g. `invalid_credentials`). VoIP.ms
    /// ships a couple of these capitalized (`Invalid_threshold`), so the
    /// case is preserved exactly as documented.
    pub code: String,
    /// Human-readable meaning from the docs.
    pub description: String,
}

/// The key under which an extracted document records the digest of the rest
/// of its content.
const DIGEST_KEY: &str = "digest";

/// Render `doc` as the pretty JSON an extract writes, with a trailing
/// [`DIGEST_KEY`] entry over the rest of its content.
fn seal(doc: &impl Serialize) -> Result<String, String> {
    let mut value = serde_json::to_value(doc).map_err(|e| format!("serialize JSON: {e}"))?;
    let digest = content_digest(&value)?;
    let JsonValue::Object(fields) = &mut value else {
        return Err("an extracted document must be a JSON object".into());
    };

    fields.insert(DIGEST_KEY.into(), JsonValue::String(digest));
    let pretty =
        serde_json::to_string_pretty(&value).map_err(|e| format!("serialize JSON: {e}"))?;
    Ok(format!("{pretty}\n"))
}

/// Check that the extracted document at `path` still matches the digest its
/// extract recorded. A mismatch means the file was edited by hand, and the
/// next run of `cargo xtask <extract_cmd>` would silently drop that edit. A
/// missing file passes; its readers decide what that means.
pub fn verify_digest(path: &Path, extract_cmd: &str) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    let text = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    match matches_digest(&text) {
        Ok(true) => Ok(()),
        Ok(false) => Err(format!(
            "{} was edited after `cargo xtask {extract_cmd}` wrote it, and the next \
             extract would drop that edit; restore the file and put the correction in \
             tools/api-response-overrides.json",
            path.display()
        )),
        Err(e) => Err(format!(
            "{}: {e}, so `cargo xtask {extract_cmd}` did not write it",
            path.display()
        )),
    }
}

/// Whether sealed `text` still matches its recorded digest. Fails when the
/// text is not JSON or records no digest.
fn matches_digest(text: &str) -> Result<bool, String> {
    let mut value: JsonValue = serde_json::from_str(text).map_err(|e| format!("parse: {e}"))?;
    let recorded = match &mut value {
        JsonValue::Object(fields) => fields.shift_remove(DIGEST_KEY),
        _ => None,
    };

    let Some(JsonValue::String(recorded)) = recorded else {
        return Err(format!("no `{DIGEST_KEY}`"));
    };

    Ok(content_digest(&value)? == recorded)
}

/// FNV-1a (64-bit) over the compact JSON form of `value`, as 16 hex digits.
/// It catches an accidental edit, not a deliberate forgery, and formatting
/// alone does not change it.
fn content_digest(value: &JsonValue) -> Result<String, String> {
    let compact = serde_json::to_string(value).map_err(|e| format!("serialize JSON: {e}"))?;
    let hash = compact
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        });

    Ok(format!("{hash:016x}"))
}

/// Extract the global "Error Codes" table from the saved API doc HTML into
/// `tools/api-statuses.json`. The table is a flat `code → description`
/// listing shared across every method (the docs do not map statuses to
/// individual methods), so the output is a single ordered list.
pub fn cmd_extract_statuses(html_path: &Path, out_path: &Path) -> Result<(), String> {
    let bytes = fs::read(html_path).map_err(|e| format!("read {}: {e}", html_path.display()))?;
    let html = String::from_utf8_lossy(&bytes).into_owned();

    let statuses = scan_status_table(&html);
    if statuses.is_empty() {
        return Err("no `Error Codes` table found in HTML — \
                    has the doc layout changed?"
            .to_string());
    }

    let doc = StatusDocument {
        version: 1,
        statuses,
    };
    fs::write(out_path, seal(&doc)?).map_err(|e| format!("write {}: {e}", out_path.display()))?;

    println!(
        "wrote {} ({} status codes)",
        out_path.display(),
        doc.statuses.len(),
    );
    Ok(())
}

/// Scan the `Error Codes` table: the two-column rows that follow the
/// `toptitlex normaltextbold` cell whose text is `Error Codes`. Each row
/// is `<td …linefull…>CODE</td><td …linerightfull…>DESCRIPTION</td>`.
///
/// Those two cell classes are also used by per-method `Parameters`/`Output`
/// rows elsewhere, so we anchor on the `Error Codes` title and stop at the
/// next `toptitlex` section title to capture exactly the one table.
fn scan_status_table(html: &str) -> Vec<StatusEntry> {
    const TITLE_MARK: &str = "toptitlex normaltextbold";
    const CODE_MARK: &str = "leftmenubottomtdlinefull normaltext";
    const DESC_MARK: &str = "leftmenubottomtdlinerightfull normaltext";

    // The whole table lives on a contiguous run of lines after the title;
    // join into one string so a `<tr>` split across lines still matches.
    let Some(title_pos) = find_error_codes_title(html, TITLE_MARK) else {
        return Vec::new();
    };
    let region = &html[title_pos..];

    let mut out: Vec<StatusEntry> = Vec::new();
    let mut rest = region;
    loop {
        // Stop at the next section title — the error table is the last one
        // in its `<table>`, so a following `toptitlex` means we've left it.
        let next_title = rest.find(TITLE_MARK);
        let Some(code_at) = rest.find(CODE_MARK) else {
            break;
        };
        if next_title.is_some_and(|t| t < code_at) {
            break;
        }

        // Advance past the code cell's opening `>` to its text.
        let after_code_open = match rest[code_at..].find('>') {
            Some(g) => code_at + g + 1,
            None => break,
        };
        let code_end = match rest[after_code_open..].find("</td>") {
            Some(e) => after_code_open + e,
            None => break,
        };
        let code = clean_cell(&rest[after_code_open..code_end]);

        // The description cell must immediately follow.
        let desc_mark_at = match rest[code_end..].find(DESC_MARK) {
            Some(d) => code_end + d,
            None => break,
        };
        let after_desc_open = match rest[desc_mark_at..].find('>') {
            Some(g) => desc_mark_at + g + 1,
            None => break,
        };
        let desc_end = match rest[after_desc_open..].find("</td>") {
            Some(e) => after_desc_open + e,
            None => break,
        };
        let description = clean_cell(&rest[after_desc_open..desc_end]);

        if !code.is_empty() && !description.is_empty() {
            out.push(StatusEntry { code, description });
        }

        rest = &rest[desc_end..];
    }

    out
}

/// Find the byte offset just past the `Error Codes` title cell's class
/// marker, so scanning starts inside that section.
fn find_error_codes_title(html: &str, title_mark: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(rel) = html[from..].find(title_mark) {
        let mark_at = from + rel;
        let after = mark_at + title_mark.len();
        // Cell text runs from the next `>` to the closing `</td>`.
        if let Some(g) = html[after..].find('>') {
            let text_start = after + g + 1;
            if let Some(e) = html[text_start..].find("</td>")
                && clean_cell(&html[text_start..text_start + e]) == "Error Codes"
            {
                return Some(after);
            }
        }
        from = after;
    }
    None
}

/// Strip inline tags, decode entities, and collapse whitespace in a table
/// cell's inner HTML.
fn clean_cell(raw: &str) -> String {
    collapse_ws(&html_decode(&strip_tags(raw)))
}

/// Scan the doc's method-description rows for per-method one-line summaries.
///
/// These use the same two-column rows as the error table -- a `linefull` cell
/// holding the name and a `linerightfull` cell holding the description -- but
/// here the left
/// cell holds a camelCase wire method name. We keep only rows whose name is
/// a known WSDL operation (so section headers and the error table are
/// ignored), take the first description seen per method, and preserve the
/// `<br>`-delimited bullet lines as newlines. Most methods carry a
/// description (~218 of 222); the rest are simply absent.
fn scan_method_docs(html: &str, known: &BTreeSet<&str>) -> serde_json::Map<String, JsonValue> {
    const CODE_MARK: &str = "leftmenubottomtdlinefull normaltext";
    const DESC_MARK: &str = "leftmenubottomtdlinerightfull normaltext";

    let mut out = serde_json::Map::new();
    let mut rest = html;
    while let Some(code_at) = rest.find(CODE_MARK) {
        let after_code_open = match rest[code_at..].find('>') {
            Some(g) => code_at + g + 1,
            None => break,
        };
        let code_end = match rest[after_code_open..].find("</td>") {
            Some(e) => after_code_open + e,
            None => break,
        };
        let name = clean_cell(&rest[after_code_open..code_end]);

        // The description cell must immediately follow this code cell, with
        // nothing but whitespace/markup in between.
        let desc = rest[code_end..]
            .find(DESC_MARK)
            .map(|d| code_end + d)
            .and_then(|mark_at| rest[mark_at..].find('>').map(|g| mark_at + g + 1))
            .and_then(|start| {
                rest[start..]
                    .find("</td>")
                    .map(|e| clean_method_desc(&rest[start..start + e]))
            });

        if known.contains(name.as_str())
            && let Some(desc) = desc
            && !desc.is_empty()
            && !out.contains_key(&name)
        {
            out.insert(name, JsonValue::String(desc));
        }

        // Advance past this code cell to find the next row.
        rest = &rest[code_end..];
    }

    out
}

/// Clean a TOC description cell: `<br>` becomes a newline (the source uses
/// it to separate bullet-style clauses), other tags are dropped, entities
/// decoded, and each line's internal whitespace collapsed. Leading
/// `- ` bullet markers are kept verbatim — `render_doc` escapes them so they
/// don't render as Markdown lists.
fn clean_method_desc(raw: &str) -> String {
    let with_breaks = raw.replace("<br>", "\n").replace("<br/>", "\n");
    let stripped = html_decode(&strip_tags(&with_breaks));
    stripped
        .lines()
        .map(collapse_ws)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn cmd_extract_responses(html_path: &Path, out_path: &Path) -> Result<(), String> {
    // The saved HTML occasionally contains stray Windows-1252 bytes
    // (smart quotes inside example text), so decode lossily rather than
    // failing the whole extract.
    let bytes = fs::read(html_path).map_err(|e| format!("read {}: {e}", html_path.display()))?;
    let html = String::from_utf8_lossy(&bytes).into_owned();

    // Cross-check method names against the WSDL so we ignore TOC entries
    // and section headers like "General"/"Accounts" naturally.
    let wsdl_path = crate::repo_root().join("tools").join("server.wsdl");
    let wsdl_text =
        fs::read_to_string(&wsdl_path).map_err(|e| format!("read {}: {e}", wsdl_path.display()))?;
    let wsdl = wsdl::parse_wsdl(&wsdl_text)?;
    let known: BTreeSet<&str> = wsdl.operations.iter().map(String::as_str).collect();

    let method_docs = scan_method_docs(&html, &known);

    let blocks = scan_html(&html);
    let mut methods = serde_json::Map::new();
    let mut param_docs = serde_json::Map::new();
    let mut param_notes = serde_json::Map::new();
    let mut covered: BTreeSet<String> = BTreeSet::new();
    let mut params_covered: BTreeSet<String> = BTreeSet::new();
    for block in &blocks {
        let name = &block.name;
        if !known.contains(name.as_str()) {
            continue;
        }
        match block.kind {
            BlockKind::Output => {
                if covered.contains(name) {
                    // Same method appears in multiple sections
                    // occasionally; first definition wins.
                    continue;
                }
                let decoded = html_decode(&block.body);
                let shape = match parse_php_block(&decoded) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("warning: {name}: skipping output — parse error: {e}");
                        continue;
                    }
                };
                methods.insert(name.clone(), shape_to_json(&shape));
                covered.insert(name.clone());
            }
            BlockKind::Parameters => {
                if params_covered.contains(name) {
                    continue;
                }
                let ParamCell { params: docs, note } = parse_param_docs(&block.body);
                if let Some(note) = note {
                    param_notes.insert(name.clone(), JsonValue::String(note));
                }

                if !docs.is_empty() {
                    let map: serde_json::Map<String, JsonValue> = docs
                        .into_iter()
                        .map(|(k, v)| (k, JsonValue::String(v)))
                        .collect();
                    param_docs.insert(name.clone(), JsonValue::Object(map));
                    params_covered.insert(name.clone());
                }
            }
        }
    }

    let param_doc_count: usize = param_docs
        .values()
        .filter_map(|v| v.as_object())
        .map(serde_json::Map::len)
        .sum();

    let method_doc_count = method_docs.len();
    let doc = Document {
        version: 1,
        methods,
        param_docs,
        param_notes,
        method_docs,
    };
    fs::write(out_path, seal(&doc)?).map_err(|e| format!("write {}: {e}", out_path.display()))?;

    let missing: Vec<&&str> = known.iter().filter(|op| !covered.contains(**op)).collect();
    println!(
        "wrote {} ({} methods covered, {} missing; {} param descriptions across \
         {} methods; {} method descriptions)",
        out_path.display(),
        covered.len(),
        missing.len(),
        param_doc_count,
        params_covered.len(),
        method_doc_count,
    );
    if !missing.is_empty() {
        let preview: Vec<&&&str> = missing.iter().take(10).collect();
        eprintln!(
            "warning: {} methods missing output blocks: {preview:?}",
            missing.len()
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// HTML scanning
// ---------------------------------------------------------------------------

/// Which labelled cell a captured `<pre>` block came from.
enum BlockKind {
    Parameters,
    Output,
}

/// A captured `<pre>` block: the method it belongs to, which cell it
/// came from, and its raw (still HTML-encoded) text.
struct Block {
    name: String,
    kind: BlockKind,
    body: String,
}

/// Walks the HTML line-by-line collecting [`Block`]s — one per
/// `Parameters`/`Output` `<pre>` cell. The HTML format is regular
/// enough that we don't need a full
/// parser: each method appears as
///
/// ```text
/// <td colspan="2" class="toptitlex normaltextbold">
///     METHOD_NAME
/// </td>
/// ...
/// <td ... class="leftmenubottomtdlinerightfull normaltext">
///     <pre><code>
///     ...output...
///     </code></pre>
/// </td>
/// ```
///
/// We capture every `<pre><code>` block and associate it with the most
/// recent `toptitlex normaltextbold` name and the preceding label cell
/// text ("Parameters" or "Output"), so the caller can route each block.
fn scan_html(html: &str) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    let mut current_name: Option<String> = None;
    let mut last_label: Option<String> = None;
    let lines: Vec<&str> = html.lines().collect();
    let n = lines.len();
    let mut i = 0;
    while i < n {
        let line = lines[i];
        if line.contains("toptitlex normaltextbold") {
            // The method name appears either inline with the opening
            // `<td …>NAME</td>` cell, or on a following line before the
            // closing tag. Try inline first.
            let inline = line
                .split_once("toptitlex normaltextbold")
                .map(|x| x.1)
                .and_then(|after| after.split_once('>'))
                .map(|(_, rest)| rest.split('<').next().unwrap_or("").trim().to_string())
                .filter(|s| !s.is_empty());

            if let Some(name) = inline {
                current_name = Some(name);
                i += 1;
                continue;
            }

            let mut j = i + 1;
            while j < n {
                let candidate = lines[j].trim();
                if candidate.is_empty() {
                    j += 1;
                    continue;
                }

                if candidate.starts_with("</td>") {
                    break;
                }

                let name = candidate.split('<').next().unwrap_or("").trim();
                if !name.is_empty() {
                    current_name = Some(name.to_string());
                }

                break;
            }

            i = j + 1;
            continue;
        }

        if line.contains("leftmenubottomtdlinefull normaltext") {
            // Label cell ("Parameters" or "Output"). The text may be
            // inline on the same line, or on the line below.
            let inline_label = line
                .split_once("leftmenubottomtdlinefull normaltext")
                .map(|x| x.1)
                .and_then(|after| after.split_once('>'))
                .map(|(_, rest)| rest.split('<').next().unwrap_or("").trim().to_string())
                .filter(|s| !s.is_empty());
            if let Some(label) = inline_label {
                last_label = Some(label);
                i += 1;
                continue;
            }

            let mut j = i + 1;
            while j < n {
                let candidate = lines[j].trim();
                if candidate.is_empty() {
                    j += 1;
                    continue;
                }

                if candidate.starts_with("</td>") {
                    break;
                }

                let trimmed = candidate.split('<').next().unwrap_or("").trim();
                last_label = Some(trimmed.to_string());
                break;
            }

            i = j + 1;
            continue;
        }
        let label_kind = match last_label.as_deref() {
            Some("Output") => Some(BlockKind::Output),
            Some("Parameters") => Some(BlockKind::Parameters),
            _ => None,
        };
        if line.contains("<pre")
            && let Some(kind) = label_kind
        {
            // Some cells use `<pre><code>…</code></pre>`, others bare
            // `<pre>…</pre>`. Detect which opener this line carries.
            let (open_tag, close_tag) = if line.contains("<pre><code>") {
                ("<pre><code>", "</code></pre>")
            } else {
                ("<pre>", "</pre>")
            };
            let mut body = String::new();
            if let Some((_, tail)) = line.split_once(open_tag) {
                if let Some(end) = tail.find(close_tag) {
                    body.push_str(&tail[..end]);
                    if let Some(name) = current_name.clone() {
                        out.push(Block { name, kind, body });
                    }

                    i += 1;
                    last_label = None;
                    continue;
                }

                body.push_str(tail);
                body.push('\n');
            }

            i += 1;
            while i < n {
                let l = lines[i];
                if let Some(end) = l.find(close_tag) {
                    body.push_str(&l[..end]);
                    break;
                }

                body.push_str(l);
                body.push('\n');
                i += 1;
            }

            if let Some(name) = current_name.clone() {
                out.push(Block { name, kind, body });
            }

            last_label = None;
            i += 1;
            continue;
        }

        i += 1;
    }

    out
}

fn html_decode(s: &str) -> String {
    s.replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&quot;", "\"")
        .replace("&#039;", "'")
        .replace("&apos;", "'")
        .replace("&#160;", " ")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
}

// ---------------------------------------------------------------------------
// Parameter-description parser
// ---------------------------------------------------------------------------

/// A method's `Parameters` cell: the per-parameter descriptions and the
/// note that follows the table, if any.
#[derive(Debug, Default, PartialEq)]
struct ParamCell {
    /// `(param_name, description)` pairs in the order the docs list them.
    params: Vec<(String, String)>,
    /// The paragraph after the table, which describes the method's
    /// parameters as a group (the routing-string format, a `members`
    /// grammar) rather than any one of them.
    note: Option<String>,
}

/// Parse a method's `Parameters` cell into its descriptions and trailing
/// note.
///
/// Each parameter starts a line `name => description`; the arrow is
/// surrounded by arbitrary whitespace. An indented line that doesn't start a
/// new parameter is continuation text belonging to the previous one, and so is
/// a flush-left line opening with `(`, the one way the docs wrap a
/// description onto the next line. Any other flush-left line that is not a
/// parameter ends the table: it and everything after it is the note, which
/// the docs set flush left so it reads under the whole table. Without that rule the note ran on into
/// the last parameter's description. The `api_username`/`api_password` rows
/// (present on most methods) are dropped -- they come from the `Client`, not
/// the request struct.
///
/// `raw` is the still-HTML-encoded cell body. Inline tags (e.g. the
/// Cloudflare email-obfuscation `<a>`) are stripped, entities decoded,
/// and whitespace collapsed before light normalization.
fn parse_param_docs(raw: &str) -> ParamCell {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut note: Vec<String> = Vec::new();
    for line in raw.lines() {
        let cleaned = collapse_ws(&unescape_slashes(&html_decode(&strip_tags(line))));
        if cleaned.is_empty() {
            continue;
        }

        if !note.is_empty() {
            note.push(cleaned);
            continue;
        }

        match split_param_line(&cleaned) {
            Some((name, desc)) => out.push((name.to_string(), desc.to_string())),
            None if !line.starts_with(char::is_whitespace)
                && !cleaned.starts_with('(')
                && !out.is_empty() =>
            {
                note.push(cleaned)
            }
            None => {
                // Continuation of the previous parameter's description.
                if let Some(last) = out.last_mut() {
                    if !last.1.is_empty() && !cleaned.is_empty() {
                        last.1.push(' ');
                    }
                    last.1.push_str(&cleaned);
                }
            }
        }
    }

    let params = out
        .into_iter()
        .filter(|(name, _)| !crate::CLIENT_FIELDS.contains(&name.as_str()))
        .map(|(name, desc)| (name, normalize_desc(&desc)))
        .filter(|(_, desc)| !desc.is_empty())
        .collect();
    let note = (!note.is_empty()).then(|| note.join(" "));
    ParamCell { params, note }
}

/// If `line` begins a `name => description` parameter row, return the
/// split. Only treats it as a new parameter when the text before the
/// first `=>` is a single identifier-shaped token — so an arrow that
/// appears inside prose continuation text isn't mistaken for a new row.
fn split_param_line(line: &str) -> Option<(&str, &str)> {
    let (lhs, rhs) = line.split_once("=>")?;
    let name = lhs.trim();
    if name.is_empty() || name.contains(char::is_whitespace) {
        return None;
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    if !name.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return None;
    }

    Some((name, rhs.trim()))
}

/// Undo stray backslash escapes that leaked from the source's PHP/JS
/// string literals into the docs text (`don\'t` → `don't`). Only the
/// quote escapes are unwound; a backslash before anything else is kept.
fn unescape_slashes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && matches!(chars.peek(), Some('\'') | Some('"')) {
            continue;
        }

        out.push(c);
    }

    out
}

/// Drop inline HTML tags, keeping their text content.
fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut depth = 0usize;
    for c in s.chars() {
        match c {
            '<' => depth += 1,
            '>' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }

    out
}

/// Collapse runs of whitespace to single spaces and trim the ends.
fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Lightly normalize a raw description into prose suitable for a Rust
/// doc comment, without editorializing the wording:
///
/// * a leading `[Required]`/`[Optional]` marker becomes a trailing
///   `(required)` / `(optional)` note;
/// * a leading `- ` is dropped: the docs use it as a column separator, so a
///   description with no lead sentence starts with one;
/// * a `Values:` label gains a following space (`Values:1=Enable` →
///   `Values: 1=Enable`) for readability.
///
/// Other punctuation — examples, `=`-laden value lists, URLs — is left
/// untouched so the rendered text stays faithful to the source.
fn normalize_desc(desc: &str) -> String {
    let mut text = collapse_ws(desc.trim());

    let mut suffix = "";
    for (marker, note) in [("[Required]", " (required)"), ("[Optional]", " (optional)")] {
        if let Some(rest) = text.strip_prefix(marker) {
            text = collapse_ws(rest.trim_start());
            suffix = note;
            break;
        }
    }

    if let Some(rest) = text.strip_prefix("- ") {
        text = rest.to_string();
    }

    // Tidy `Values:1=Enable` → `Values: 1=Enable`; leave the list itself
    // alone.
    let text = text.replace("Values:", "Values: ");
    let mut text = collapse_ws(&text);
    text.push_str(suffix);
    text
}

// ---------------------------------------------------------------------------
// PHP print_r parser
// ---------------------------------------------------------------------------

/// Parse the contents of a `<pre><code>` Output block.
///
/// Expected shape:
/// ```text
/// Array
/// (
///     [key] => scalar
///     [key2] => Array
///         (
///             [nested] => value
///         )
/// )
/// ```
pub fn parse_php_block(text: &str) -> Result<Shape, String> {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let mut idx = 0;
    let shape = parse_array(&lines, &mut idx)?;
    Ok(shape)
}

fn parse_array(lines: &[&str], idx: &mut usize) -> Result<Shape, String> {
    // Expect "Array" then "(" lines.
    skip_to_keyword(lines, idx, "Array")
        .ok_or_else(|| "expected `Array` token at top of block".to_string())?;
    *idx += 1;
    skip_to_keyword(lines, idx, "(").ok_or_else(|| "expected `(` after `Array`".to_string())?;
    *idx += 1;
    parse_body(lines, idx)
}

fn skip_to_keyword(lines: &[&str], idx: &mut usize, kw: &str) -> Option<()> {
    while *idx < lines.len() {
        if lines[*idx].trim() == kw {
            return Some(());
        }

        *idx += 1;
    }

    None
}

fn parse_body(lines: &[&str], idx: &mut usize) -> Result<Shape, String> {
    // Read entries until we hit the matching `)` at the current depth.
    let mut entries: Vec<(String, Shape)> = Vec::new();
    while *idx < lines.len() {
        let line = lines[*idx];
        let trimmed = line.trim();
        if trimmed == ")" {
            *idx += 1;
            break;
        }

        // `[key] => value` or `[key] => Array`
        let (key, rhs) =
            parse_entry_line(trimmed).ok_or_else(|| format!("unrecognized line `{trimmed}`"))?;
        *idx += 1;
        if rhs.trim() == "Array" {
            // Next non-empty line should be `(`, then a body.
            skip_to_keyword(lines, idx, "(")
                .ok_or_else(|| format!("expected `(` after `[{key}] => Array`"))?;
            *idx += 1;
            let inner = parse_body(lines, idx)?;
            entries.push((key, inner));
        } else {
            // Possibly multi-line scalar: gobble continuation lines
            // until we hit another `[key] =>` or a `)`.
            let mut value = rhs.to_string();
            while *idx < lines.len() {
                let t = lines[*idx].trim();
                if t == ")" || is_entry_line(t) {
                    break;
                }
                value.push(' ');
                value.push_str(t);
                *idx += 1;
            }

            entries.push((
                key,
                Shape::Scalar {
                    ty: infer_scalar(&value),
                    sample: value,
                },
            ));
        }
    }

    // If keys look like sequential integer indices, treat as a list and
    // merge element shapes.
    if !entries.is_empty() && entries.iter().all(|(k, _)| k.parse::<usize>().is_ok()) {
        let mut merged: Option<Shape> = None;
        for (_, s) in &entries {
            merged = Some(match merged {
                None => s.clone(),
                Some(prev) => merge_shapes(prev, s.clone()),
            });
        }

        return Ok(Shape::List(Box::new(
            merged.unwrap_or(Shape::Object(Vec::new())),
        )));
    }

    Ok(Shape::Object(entries))
}

fn parse_entry_line(s: &str) -> Option<(String, &str)> {
    if !s.starts_with('[') {
        return None;
    }

    let close = s.find(']')?;
    let key = s[1..close].to_string();
    let rest = s[close + 1..].trim_start();
    let arrow = rest.strip_prefix("=>")?.trim_start();
    Some((key, arrow))
}

fn is_entry_line(s: &str) -> bool {
    parse_entry_line(s).is_some()
}

// ---------------------------------------------------------------------------
// Scalar type inference & shape merging
// ---------------------------------------------------------------------------

fn infer_scalar(s: &str) -> ScalarTy {
    let t = s.trim();
    if t.is_empty() {
        return ScalarTy::Empty;
    }

    if is_datetime(t) {
        return ScalarTy::DateTime;
    }

    if is_date(t) {
        return ScalarTy::Date;
    }

    if is_integer(t) {
        // "0"/"1" alone is ambiguous; classify as Bool01 only when paired
        // with another sample via merge. Default to Integer here.
        return ScalarTy::Integer;
    }

    if is_decimal(t) {
        return ScalarTy::Decimal;
    }

    match t.to_ascii_lowercase().as_str() {
        "yes" | "no" => ScalarTy::BoolYn,
        _ => ScalarTy::String,
    }
}

fn is_integer(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return false;
    }

    let (start, allow_zero_prefix) = match bytes[0] {
        b'-' | b'+' => (1, false),
        _ => (0, false),
    };

    if start >= bytes.len() {
        return false;
    }

    // Reject phone-number-looking strings with leading zeros (other than
    // bare "0"): "0123" should stay a string.
    if !allow_zero_prefix && bytes.len() - start > 1 && bytes[start] == b'0' {
        return false;
    }
    bytes[start..].iter().all(|b| b.is_ascii_digit())
}

fn is_decimal(s: &str) -> bool {
    let mut saw_dot = false;
    let mut saw_digit = false;
    let bytes = s.as_bytes();
    let mut i = 0;
    if !bytes.is_empty() && (bytes[0] == b'-' || bytes[0] == b'+') {
        i = 1;
    }

    while i < bytes.len() {
        match bytes[i] {
            b'.' if !saw_dot => saw_dot = true,
            b'0'..=b'9' => saw_digit = true,
            _ => return false,
        }
        i += 1;
    }

    saw_digit && saw_dot
}

fn is_date(s: &str) -> bool {
    s.len() == 10
        && s.as_bytes()[4] == b'-'
        && s.as_bytes()[7] == b'-'
        && s.bytes().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }
        })
}

fn is_datetime(s: &str) -> bool {
    s.len() == 19
        && is_date(&s[..10])
        && s.as_bytes()[10] == b' '
        && s.as_bytes()[13] == b':'
        && s.as_bytes()[16] == b':'
}

fn merge_shapes(a: Shape, b: Shape) -> Shape {
    match (a, b) {
        (Shape::Scalar { ty: t1, sample: s1 }, Shape::Scalar { ty: t2, sample: s2 }) => {
            let ty = merge_scalar(t1, t2, &s1, &s2);
            // Keep the first non-empty sample for clarity.
            let sample = if !s1.trim().is_empty() { s1 } else { s2 };
            Shape::Scalar { ty, sample }
        }
        (Shape::Object(mut left), Shape::Object(right)) => {
            // Union fields, preserving left order; new fields appended.
            for (k, v) in right {
                if let Some(slot) = left.iter_mut().find(|(lk, _)| lk == &k) {
                    let prev = std::mem::replace(
                        &mut slot.1,
                        Shape::Scalar {
                            ty: ScalarTy::Empty,
                            sample: String::new(),
                        },
                    );

                    slot.1 = merge_shapes(prev, v);
                } else {
                    left.push((k, v));
                }
            }
            Shape::Object(left)
        }
        (Shape::List(a), Shape::List(b)) => Shape::List(Box::new(merge_shapes(*a, *b))),
        // Mismatched node kinds: fall back to scalar/string.
        (a, _) => a,
    }
}

fn merge_scalar(a: ScalarTy, b: ScalarTy, sa: &str, sb: &str) -> ScalarTy {
    if a == b {
        return a;
    }

    use ScalarTy::*;
    match (a, b) {
        (Empty, x) | (x, Empty) => x,
        // 0/1 with yes/no never happens; integer pair "0"/"1" → bool_01.
        (Integer, Integer) => Integer,
        (Integer, Decimal) | (Decimal, Integer) => Decimal,
        (BoolYn, BoolYn) => BoolYn,
        (Bool01, Bool01) => Bool01,
        _ => {
            // Promote 0/1-only integers to Bool01 if both samples are 0 or 1.
            let only_01 = |s: &str| matches!(s.trim(), "0" | "1");
            if only_01(sa) && only_01(sb) {
                Bool01
            } else {
                String
            }
        }
    }
}

// ---------------------------------------------------------------------------
// JSON emission
// ---------------------------------------------------------------------------

fn shape_to_json(shape: &Shape) -> JsonValue {
    match shape {
        Shape::Scalar { ty, sample } => {
            let mut obj = serde_json::Map::new();
            obj.insert("kind".into(), JsonValue::String("scalar".into()));
            obj.insert("type".into(), JsonValue::String(ty.as_str().into()));
            obj.insert("sample".into(), JsonValue::String(sample.clone()));
            JsonValue::Object(obj)
        }
        Shape::Object(fields) => {
            let mut obj = serde_json::Map::new();
            obj.insert("kind".into(), JsonValue::String("object".into()));
            let mut field_array = Vec::with_capacity(fields.len());
            for (name, sub) in fields {
                let mut entry = serde_json::Map::new();
                entry.insert("name".into(), JsonValue::String(name.clone()));
                entry.insert("shape".into(), shape_to_json(sub));
                field_array.push(JsonValue::Object(entry));
            }
            obj.insert("fields".into(), JsonValue::Array(field_array));
            JsonValue::Object(obj)
        }
        Shape::List(inner) => {
            let mut obj = serde_json::Map::new();
            obj.insert("kind".into(), JsonValue::String("list".into()));
            obj.insert("element".into(), shape_to_json(inner));
            JsonValue::Object(obj)
        }
        Shape::Map(value) => {
            let mut obj = serde_json::Map::new();
            obj.insert("kind".into(), JsonValue::String("map".into()));
            obj.insert("value".into(), shape_to_json(value));
            JsonValue::Object(obj)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(cell: &ParamCell) -> Vec<(&str, &str)> {
        cell.params
            .iter()
            .map(|(n, d)| (n.as_str(), d.as_str()))
            .collect()
    }

    #[test]
    fn flush_left_text_after_the_table_is_the_note_not_the_last_param() {
        let raw = [
            "",
            "did     => [Required] DID to be Ordered",
            "routing => [Required] Main Routing for the DID",
            "test    =>            Set to true if testing how Orders work",
            "                      - Orders can not be undone",
            "",
            "routing can receive values in the following format => header:record_id",
            "",
            "    vm          Used for routing calls to a Voicemail",
        ]
        .join("\n");
        let cell = parse_param_docs(&raw);
        assert_eq!(
            pairs(&cell),
            [
                ("did", "DID to be Ordered (required)"),
                ("routing", "Main Routing for the DID (required)"),
                (
                    "test",
                    "Set to true if testing how Orders work - Orders can not be undone"
                ),
            ]
        );
        assert_eq!(
            cell.note.as_deref(),
            Some(
                "routing can receive values in the following format => header:record_id \
                 vm Used for routing calls to a Voicemail"
            )
        );
    }

    #[test]
    fn a_param_after_a_blank_line_stays_in_the_table() {
        let raw = [
            "",
            "name =>  Name",
            "",
            "          more about name",
            "",
            "id   =>  Id",
        ]
        .join("\n");
        let cell = parse_param_docs(&raw);
        assert_eq!(
            pairs(&cell),
            [("name", "Name more about name"), ("id", "Id")]
        );
        assert_eq!(cell.note, None);
    }

    #[test]
    fn a_flush_left_parenthetical_continues_the_description() {
        let raw = [
            "activate           =>  Activates Client",
            "balance_management =>  Balance Management for Client ",
            "(Values from getBalanceManagement)",
        ]
        .join("\n");
        let cell = parse_param_docs(&raw);
        assert_eq!(
            pairs(&cell),
            [
                ("activate", "Activates Client"),
                (
                    "balance_management",
                    "Balance Management for Client (Values from getBalanceManagement)"
                ),
            ]
        );
        assert_eq!(cell.note, None);
    }

    #[test]
    fn a_leading_dash_is_dropped() {
        assert_eq!(
            normalize_desc("- (Values: 1 = Business, 0 = Residential) - Default: 0"),
            "(Values: 1 = Business, 0 = Residential) - Default: 0"
        );
        assert_eq!(normalize_desc("[Required] - Id"), "Id (required)");
    }

    fn sealed_statuses() -> String {
        seal(&StatusDocument {
            version: 1,
            statuses: vec![StatusEntry {
                code: "no_did".into(),
                description: "There are no DIDs".into(),
            }],
        })
        .unwrap()
    }

    #[test]
    fn a_sealed_document_matches_its_digest() {
        assert_eq!(matches_digest(&sealed_statuses()), Ok(true));
    }

    #[test]
    fn an_edited_value_no_longer_matches() {
        let edited = sealed_statuses().replace("There are no DIDs", "No DIDs");
        assert_eq!(matches_digest(&edited), Ok(false));
    }

    #[test]
    fn reformatting_keeps_the_match() {
        let value: JsonValue = serde_json::from_str(&sealed_statuses()).unwrap();
        let compact = serde_json::to_string(&value).unwrap();
        assert_eq!(matches_digest(&compact), Ok(true));
    }

    #[test]
    fn a_document_without_a_digest_is_an_error() {
        let unsealed = serde_json::to_string(&StatusDocument {
            version: 1,
            statuses: Vec::new(),
        })
        .unwrap();
        assert!(matches_digest(&unsealed).is_err());
    }
}
