use anyhow::{anyhow, Result};
use petgraph::{
    graph::{DiGraph, NodeIndex},
    visit::DfsPostOrder,
    Direction,
};
use std::{
    collections::{btree_map::Entry, BTreeMap, HashMap},
    fs::{create_dir_all, read_to_string, write},
    iter::repeat,
    path::{Component, Path, PathBuf},
};

use super::html::{
    CurrentView, DirectoryPage, FilePage, FunctionListing, Head, HtmlFunctionInfo, HtmlLineInfo,
    HtmlSummaryInfo, Listing, Page, Summary,
};

#[derive(Debug, Clone, Default)]
pub struct TestNameRecordEntry(String);

impl std::fmt::Display for TestNameRecordEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TN:{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct SourceFileRecordEntry(PathBuf);

impl std::fmt::Display for SourceFileRecordEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SF:{}", self.0.to_string_lossy())
    }
}

impl Default for SourceFileRecordEntry {
    fn default() -> Self {
        Self(PathBuf::new())
    }
}

#[derive(Debug, Clone)]
pub struct VersionRecordEntry(usize);

impl Default for VersionRecordEntry {
    fn default() -> Self {
        Self(1)
    }
}

impl std::fmt::Display for VersionRecordEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VER:{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct FunctionRecordEntry {
    start_line: usize,
    end_line: Option<usize>,
    name: String,
}

impl std::fmt::Display for FunctionRecordEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.end_line {
            Some(end_line) => write!(f, "FN:{},{},{}", self.start_line, end_line, self.name),
            None => write!(f, "FN:{},{}", self.start_line, self.name),
        }
    }
}

impl std::cmp::PartialEq for FunctionRecordEntry {
    fn eq(&self, other: &Self) -> bool {
        self.start_line == other.start_line && self.name == other.name
    }
}

impl std::cmp::PartialOrd for FunctionRecordEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Entries are ordered by start line
        self.start_line.partial_cmp(&other.start_line)
    }
}

#[derive(Debug, Clone)]
pub struct FunctionDataRecordEntry {
    hits: usize,
    name: String,
}

impl std::fmt::Display for FunctionDataRecordEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FNDA:{},{}", self.hits, self.name)
    }
}

impl std::cmp::PartialEq for FunctionDataRecordEntry {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

#[derive(Debug, Clone, Default)]
pub struct FunctionsFoundRecordEntry(usize);

impl std::fmt::Display for FunctionsFoundRecordEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FNF:{}", self.0)
    }
}

#[derive(Debug, Clone, Default)]
pub struct FunctionsHitRecordEntry(usize);

impl std::fmt::Display for FunctionsHitRecordEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FNH:{}", self.0)
    }
}

// NOTE: We don't bother to implement branch data since we have the line data anyway

// BRDA, BRF, BRH empty

#[derive(Debug, Clone)]
pub struct LineRecordEntry {
    line_number: usize,
    hit_count: usize,
    /// MD5 hash of line saved as base64, typically not used
    checksum: Option<String>,
}

impl std::fmt::Display for LineRecordEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.checksum {
            Some(checksum) => {
                write!(f, "DA:{},{},{}", self.line_number, self.hit_count, checksum)
            }
            None => write!(f, "DA:{},{}", self.line_number, self.hit_count),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LinesFoundRecordEntry(usize);

impl std::fmt::Display for LinesFoundRecordEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LF:{}", self.0)
    }
}

#[derive(Debug, Clone, Default)]
pub struct LinesHitRecordEntry(usize);

impl std::fmt::Display for LinesHitRecordEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LH:{}", self.0)
    }
}

#[derive(Debug, Clone, Default)]
pub struct EndOfRecordEntry;

impl std::fmt::Display for EndOfRecordEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "end_of_record")
    }
}

#[derive(Debug, Clone, Default)]
pub struct Record {
    test_name: TestNameRecordEntry,
    source_file: SourceFileRecordEntry,
    version: VersionRecordEntry,
    // Functions ordered by start line
    functions: BTreeMap<usize, FunctionRecordEntry>,
    // Function datas are unique
    function_data: HashMap<String, FunctionDataRecordEntry>,
    functions_found: FunctionsFoundRecordEntry,
    functions_hit: FunctionsHitRecordEntry,
    // Lines are ordered by line and unique
    lines: BTreeMap<usize, LineRecordEntry>,
    lines_found: LinesFoundRecordEntry,
    lines_hit: LinesHitRecordEntry,
    end_of_record: EndOfRecordEntry,
}

impl Record {
    pub fn new<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            source_file: SourceFileRecordEntry(path.as_ref().to_path_buf()),
            functions: BTreeMap::new(),
            function_data: HashMap::new(),
            lines: BTreeMap::new(),
            ..Default::default()
        }
    }

    pub fn add_function_if_not_exists<S>(
        &mut self,
        start_line: usize,
        end_line: Option<usize>,
        name: S,
    ) -> bool
    where
        S: AsRef<str>,
    {
        match self.functions.entry(start_line) {
            Entry::Occupied(_) => false,
            Entry::Vacant(entry) => {
                entry.insert(FunctionRecordEntry {
                    start_line,
                    end_line,
                    name: name.as_ref().to_string(),
                });
                self.functions_found.0 += 1;
                self.function_data
                    .entry(name.as_ref().to_string())
                    .or_insert_with(|| FunctionDataRecordEntry {
                        hits: 0,
                        name: name.as_ref().to_string(),
                    });
                true
            }
        }
    }

    pub fn increment_function_data<S>(&mut self, name: S)
    where
        S: AsRef<str>,
    {
        let entry = self
            .function_data
            .entry(name.as_ref().to_string())
            .or_insert_with(|| FunctionDataRecordEntry {
                hits: 0,
                name: name.as_ref().to_string(),
            });

        if entry.hits == 0 {
            self.functions_hit.0 += 1;
        }

        entry.hits += 1;
    }

    pub fn add_line_if_not_exists(&mut self, line_number: usize) -> bool {
        match self.lines.entry(line_number) {
            Entry::Occupied(_) => false,
            Entry::Vacant(entry) => {
                entry.insert(LineRecordEntry {
                    line_number,
                    hit_count: 0,
                    checksum: None,
                });
                self.lines_found.0 += 1;
                true
            }
        }
    }

    pub fn increment_line(&mut self, line_number: usize) {
        let entry = self
            .lines
            .entry(line_number)
            .or_insert_with(|| LineRecordEntry {
                line_number,
                hit_count: 0,
                checksum: None,
            });

        if entry.hit_count == 0 {
            self.lines_hit.0 += 1;
        }

        entry.hit_count += 1;
    }
}

impl std::fmt::Display for Record {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.test_name)?;
        writeln!(f, "{}", self.source_file)?;
        writeln!(f, "{}", self.version)?;
        for function in self.functions.values() {
            writeln!(f, "{}", function)?;
        }
        for function_data in self.function_data.values() {
            writeln!(f, "{}", function_data)?;
        }
        writeln!(f, "{}", self.functions_found)?;
        writeln!(f, "{}", self.functions_hit)?;
        for line in self.lines.values() {
            writeln!(f, "{}", line)?;
        }
        writeln!(f, "{}", self.lines_found)?;
        writeln!(f, "{}", self.lines_hit)?;
        writeln!(f, "{}", self.end_of_record)?;
        Ok(())
    }
}

impl Record {
    pub fn source_filename(&self) -> String {
        self.source_file
            .0
            .file_name()
            .map(|fname| fname.to_string_lossy().to_string())
            .unwrap_or("<unknown>".to_string())
    }

    pub fn summary(&self, top_level: PathBuf, parent: Option<PathBuf>) -> HtmlSummaryInfo {
        HtmlSummaryInfo {
            is_dir: false,
            top_level,
            parent,
            filename: Some(self.source_filename()),
            total_lines: self.lines_found.0,
            hit_lines: self.lines_hit.0,
            total_functions: self.functions_found.0,
            hit_functions: self.functions_hit.0,
        }
    }

    pub fn lines(&self) -> Result<Vec<HtmlLineInfo>> {
        let contents = read_to_string(self.source_file.0.as_path())?;
        let lines = contents
            .lines()
            .enumerate()
            .map(|(i, line)| {
                let hit_count = self.lines.get(&(i + 1)).map(|l| l.hit_count);
                let leading_spaces = line.chars().take_while(|c| c.is_whitespace()).count();
                let trimmed = line.trim().to_string();
                HtmlLineInfo {
                    hit_count,
                    leading_spaces,
                    line: trimmed,
                }
            })
            .collect::<Vec<_>>();
        Ok(lines)
    }

    pub fn functions(&self) -> Vec<HtmlFunctionInfo> {
        let mut functions = self
            .functions
            .values()
            .map(|f| HtmlFunctionInfo {
                hit_count: self.function_data.get(&f.name).map(|d| d.hits),
                name: f.name.as_str().to_string(),
            })
            .collect::<Vec<_>>();
        functions.sort_by(|a, b| a.name.cmp(&b.name));
        functions
    }
}

#[derive(Debug, Clone, Default)]
pub struct Records(HashMap<PathBuf, Record>);

impl Records {
    pub fn get_or_insert_mut<P>(&mut self, path: P) -> &mut Record
    where
        P: AsRef<Path>,
    {
        self.0
            .entry(path.as_ref().to_path_buf())
            .or_insert_with(|| Record::new(path))
    }

    pub fn get(&self, path: &Path) -> Option<&Record> {
        self.0.get(path)
    }
}

impl std::fmt::Display for Records {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for record in self.0.values() {
            write!(f, "{}", record)?;
        }
        Ok(())
    }
}

impl Records {
    /// Output LCOV records to HTML format, like a mini genhtml
    pub fn to_html<P>(&self, output_directory: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        // Build a tree out of the output paths
        let mut graph = DiGraph::<(PathBuf, Option<HtmlSummaryInfo>), ()>::new();
        let mut node_ids = HashMap::<PathBuf, NodeIndex>::new();

        let entries = self
            .0
            .values()
            .map(|record| {
                let absolute_source_path = record.source_file.0.canonicalize()?;
                let mut output_path = output_directory
                    .as_ref()
                    .components()
                    .chain(
                        absolute_source_path
                            .components()
                            .filter(|c| matches!(c, Component::Normal(_))),
                    )
                    .collect::<PathBuf>();
                output_path.set_file_name(
                    output_path
                        .file_name()
                        .map(|fname| fname.to_string_lossy().to_string())
                        .unwrap_or_default()
                        + ".html",
                );
                if let std::collections::hash_map::Entry::Vacant(entry) =
                    node_ids.entry(output_path.clone())
                {
                    entry.insert(graph.add_node((output_path.clone(), None)));
                }

                let mut path = output_path.as_path();
                while let Some(parent) = path.parent() {
                    if let std::collections::hash_map::Entry::Vacant(entry) =
                        node_ids.entry(parent.to_path_buf())
                    {
                        entry.insert(graph.add_node((parent.to_path_buf(), None)));
                    }

                    if graph
                        .find_edge(
                            *node_ids
                                .get(parent)
                                .ok_or_else(|| anyhow!("parent not found"))?,
                            *node_ids
                                .get(path)
                                .ok_or_else(|| anyhow!("output path not found"))?,
                        )
                        .is_none()
                    {
                        graph.add_edge(
                            *node_ids
                                .get(parent)
                                .ok_or_else(|| anyhow!("parent not found"))?,
                            *node_ids
                                .get(path)
                                .ok_or_else(|| anyhow!("output path not found"))?,
                            (),
                        );
                    }

                    path = parent;

                    if !path.is_dir() {
                        create_dir_all(path)?;
                    }

                    if path == output_directory.as_ref() {
                        break;
                    }
                }

                Ok((output_path, record))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .collect::<HashMap<_, _>>();

        let root = node_ids
            .get(output_directory.as_ref())
            .ok_or_else(|| anyhow!("root not found"))?;

        let mut traversal = DfsPostOrder::new(&graph, *root);

        while let Some(node) = traversal.next(&graph) {
            let path = graph
                .node_weight(node)
                .ok_or_else(|| anyhow!("No weight for node"))?
                .0
                .clone();
            // Calculate the depth of this path from the output directory
            if let Some(record) = entries.get(path.as_path()) {
                let depth = path
                    .components()
                    .count()
                    .saturating_sub(output_directory.as_ref().components().count())
                    .saturating_sub(1);
                // This is a file node
                let summary = record.summary(
                    repeat("..")
                        .take(depth)
                        .collect::<PathBuf>()
                        .join("index.html"),
                    path.parent().map(|p| p.join("index.html")),
                );
                graph
                    .node_weight_mut(node)
                    .ok_or_else(|| anyhow!("No weight for node"))?
                    .1 = Some(summary.clone());
                let lines = record.lines()?;
                let functions = record.functions();
                let page = Page {
                    head: Head {},
                    current_view: CurrentView {
                        summary: summary.clone(),
                    },
                    summary: Summary { summary },
                    main: FilePage {
                        listing: Listing { lines },
                        function_listing: FunctionListing { functions },
                    },
                };
                write(&path, page.to_string())?;
            } else {
                let depth = path
                    .components()
                    .count()
                    .saturating_sub(output_directory.as_ref().components().count());
                let (top_level, parent) = if path == output_directory.as_ref() {
                    // This is the root node
                    (PathBuf::from("index.html"), None)
                } else {
                    // This is a directory node
                    (
                        repeat("..")
                            .take(depth)
                            .collect::<PathBuf>()
                            .join("index.html"),
                        path.parent().map(|p| p.join("index.html")),
                    )
                };
                let (total_lines, hit_lines, total_functions, hit_functions) = graph
                    .neighbors_directed(node, Direction::Outgoing)
                    .try_fold(
                        (0, 0, 0, 0),
                        |(total_lines, hit_lines, total_functions, hit_functions), neighbor| {
                            let summary = graph
                                .node_weight(neighbor)
                                .ok_or_else(|| anyhow!("No weight for node"))?
                                .1
                                .as_ref()
                                .ok_or_else(|| anyhow!("No summary for node"))?;
                            println!("Adding neighbor {:?}", summary);
                            Ok::<(usize, usize, usize, usize), anyhow::Error>((
                                total_lines + summary.total_lines,
                                hit_lines + summary.hit_lines,
                                total_functions + summary.total_functions,
                                hit_functions + summary.hit_functions,
                            ))
                        },
                    )?;

                let summary = HtmlSummaryInfo {
                    is_dir: true,
                    top_level,
                    parent,
                    filename: path
                        .file_name()
                        .map(|fname| fname.to_string_lossy().to_string()),
                    total_lines,
                    hit_lines,
                    total_functions,
                    hit_functions,
                };

                let page = Page {
                    head: Head {},
                    current_view: CurrentView {
                        summary: summary.clone(),
                    },
                    summary: Summary {
                        summary: summary.clone(),
                    },
                    main: DirectoryPage {
                        summaries: graph
                            .neighbors_directed(node, Direction::Outgoing)
                            .filter_map(|neighbor| {
                                graph
                                    .node_weight(neighbor)
                                    .ok_or_else(|| anyhow!("No weight for node"))
                                    .ok()
                                    .and_then(|weight| weight.1.as_ref().cloned())
                            })
                            .collect(),
                    },
                };
                write(path.join("index.html"), page.to_string())?;

                graph
                    .node_weight_mut(node)
                    .ok_or_else(|| anyhow!("No weight for node"))?
                    .1 = Some(summary);
            }
        }

        // NOTE: Left for easy debugging of the directory graph
        // let dot = Dot::new(&graph);
        // write(
        //     output_directory.as_ref().join("graph.dot"),
        //     format!("{:?}", dot),
        // )?;

        Ok(())
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use super::Records;

    #[test]
    fn test_records() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut records = Records::default();
        let record_test = records.get_or_insert_mut(
            manifest_dir
                .join("tests")
                .join("rsrc")
                .join("test-lcov")
                .join("test.c"),
        );
        record_test.add_function_if_not_exists(4, Some(16), "main");
        record_test.increment_function_data("main");
        record_test.add_line_if_not_exists(4);
        record_test.add_line_if_not_exists(5);
        record_test.add_line_if_not_exists(7);
        record_test.add_line_if_not_exists(9);
        record_test.add_line_if_not_exists(11);
        record_test.add_line_if_not_exists(12);
        record_test.add_line_if_not_exists(14);
        record_test.increment_line(4);
        record_test.increment_line(5);
        record_test.increment_line(7);
        record_test.increment_line(9);
        record_test.increment_line(11);
        record_test.increment_line(14);
        let record_test2 = records.get_or_insert_mut(
            manifest_dir
                .join("tests")
                .join("rsrc")
                .join("test-lcov")
                .join("test2.c"),
        );
        record_test2.add_function_if_not_exists(1, Some(3), "x");
        record_test2.increment_function_data("x");
        record_test2.add_line_if_not_exists(1);
        record_test2.add_line_if_not_exists(2);
        record_test2.add_line_if_not_exists(3);
        record_test2.increment_line(1);
        record_test2.increment_line(2);
        record_test2.increment_line(3);
        println!("{}", records);
    }

    #[test]
    fn test_records_to_html() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut records = Records::default();

        let record_test = records.get_or_insert_mut(
            manifest_dir
                .join("tests")
                .join("rsrc")
                .join("test-lcov")
                .join("test.c"),
        );
        record_test.add_function_if_not_exists(4, Some(16), "main");
        record_test.increment_function_data("main");
        record_test.add_line_if_not_exists(4);
        record_test.add_line_if_not_exists(5);
        record_test.add_line_if_not_exists(7);
        record_test.add_line_if_not_exists(9);
        record_test.add_line_if_not_exists(11);
        record_test.add_line_if_not_exists(12);
        record_test.add_line_if_not_exists(14);
        record_test.increment_line(4);
        record_test.increment_line(5);
        record_test.increment_line(7);
        record_test.increment_line(9);
        record_test.increment_line(11);
        record_test.increment_line(14);

        let record_test = records.get_or_insert_mut(
            manifest_dir
                .join("tests")
                .join("rsrc")
                .join("test-lcov")
                .join("subdir1")
                .join("test.c"),
        );
        record_test.add_function_if_not_exists(4, Some(16), "main");
        record_test.increment_function_data("main");
        record_test.add_line_if_not_exists(4);
        record_test.add_line_if_not_exists(5);
        record_test.add_line_if_not_exists(7);
        record_test.add_line_if_not_exists(9);
        record_test.add_line_if_not_exists(11);
        record_test.add_line_if_not_exists(12);
        record_test.add_line_if_not_exists(14);
        record_test.increment_line(4);
        record_test.increment_line(5);
        record_test.increment_line(7);
        record_test.increment_line(9);
        record_test.increment_line(11);
        record_test.increment_line(14);

        let record_test = records.get_or_insert_mut(
            manifest_dir
                .join("tests")
                .join("rsrc")
                .join("test-lcov")
                .join("subdir2")
                .join("test-subdir2.c"),
        );
        record_test.add_function_if_not_exists(4, Some(16), "main");
        record_test.increment_function_data("main");
        record_test.add_line_if_not_exists(4);
        record_test.add_line_if_not_exists(5);
        record_test.add_line_if_not_exists(7);
        record_test.add_line_if_not_exists(9);
        record_test.add_line_if_not_exists(11);
        record_test.add_line_if_not_exists(12);
        record_test.add_line_if_not_exists(14);
        record_test.increment_line(4);
        record_test.increment_line(5);
        record_test.increment_line(7);
        record_test.increment_line(9);
        record_test.increment_line(11);
        record_test.increment_line(14);

        let record_test2 = records.get_or_insert_mut(
            manifest_dir
                .join("tests")
                .join("rsrc")
                .join("test-lcov")
                .join("test2.c"),
        );
        record_test2.add_function_if_not_exists(1, Some(3), "x");
        record_test2.increment_function_data("x");
        record_test2.add_line_if_not_exists(1);
        record_test2.add_line_if_not_exists(2);
        record_test2.add_line_if_not_exists(3);
        record_test2.increment_line(1);
        record_test2.increment_line(2);
        record_test2.increment_line(3);

        records
            .to_html(
                manifest_dir
                    .join("tests")
                    .join("rsrc")
                    .join("test-lcov")
                    .join("html"),
            )
            .unwrap();
    }
}
