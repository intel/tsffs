use std::path::PathBuf;

use markup::{define, Render};

#[derive(Debug, Clone)]
pub(crate) struct HtmlLineInfo {
    pub(crate) hit_count: Option<usize>,
    pub(crate) leading_spaces: usize,
    pub(crate) line: String,
}

#[derive(Debug, Clone)]
pub(crate) struct HtmlFunctionInfo {
    pub(crate) hit_count: Option<usize>,
    pub(crate) name: String,
}

#[derive(Debug, Clone)]
pub(crate) struct HtmlSummaryInfo {
    pub(crate) is_dir: bool,
    // e.g. `../../../../../index.html`
    pub(crate) top_level: PathBuf,
    // e.g. `index.html` for files, `../index.html` for directories
    pub(crate) parent: Option<PathBuf>,
    // e.g. `test.c` for files, `dir-name` for directories
    pub(crate) filename: Option<String>,
    pub(crate) total_lines: usize,
    pub(crate) hit_lines: usize,
    pub(crate) total_functions: usize,
    pub(crate) hit_functions: usize,
}

impl std::fmt::Display for HtmlSummaryInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HtmlSummaryInfo {{ is_dir: {}, top_level: {:?}, parent: {:?}, filename: {:?}, total_lines: {}, hit_lines: {}, total_functions: {}, hit_functions: {} }}",
            self.is_dir, self.top_level, self.parent, self.filename, self.total_lines, self.hit_lines, self.total_functions, self.hit_functions
        )
    }
}

define! {
    Head {
    }
    Style {

    }
    CurrentView(summary: HtmlSummaryInfo) {
        table {
            tr {
                th {
                    "Current view:"
                }
                td {
                    a[href = summary.top_level.to_string_lossy().to_string()] {
                        "Top Level"
                    }
                    @if let Some(parent) = &summary.parent {
                        @if let Some(parent_parent) = parent.parent() {
                            @markup::raw("&nbsp;")
                            "-"
                            @markup::raw("&nbsp;")
                            a[href = &parent.to_string_lossy().to_string()] {
                                @parent_parent.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or("<unknown>".to_string())
                            }
                        }
                    }
                    @markup::raw("&nbsp;")
                    "-"
                    @markup::raw("&nbsp;")
                    @summary.filename
                }
            }
            tr {
                th {
                    "Generated On"
                }
                td {
                    @chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
                }
            }
        }

    }

    Summary(
        summary: HtmlSummaryInfo,
    ) {
        table {
            tr {
                th {}
                th[style = "text-align: right;"] {
                    "Coverage"
                }
                th[style = "text-align: right;"] {
                    "Total"
                }
                th[style = "text-align: right;"] {
                    "Hit"
                }
            }
            tr {
                th[style = "text-align: right;"] {
                    "Lines"
                }
                @if (summary.hit_lines as f32 / summary.total_lines as f32)
                    * 100.0 >= 90.0 {
                    td[style = "text-align: right; background-color: #a7fc9d;"] {
                        @format!(
                            "{:.02}%",
                            (summary.hit_lines as f32 / summary.total_lines as f32)
                            * 100.0
                        )
                    }
                } else if (summary.hit_lines as f32 / summary.total_lines as f32)
                    * 100.0 >= 30.0 {
                    td[style = "text-align: right; background-color: #ffea20;"] {
                        @format!(
                            "{:.02}%",
                            (summary.hit_lines as f32 / summary.total_lines as f32)
                            * 100.0
                        )
                    }
                } else {
                    td[style = "text-align: right; background-color: #ff6230;"] {
                        @format!(
                            "{:.02}%",
                            (summary.hit_lines as f32 / summary.total_lines as f32)
                            * 100.0
                        )
                    }
                }

                td[style = "text-align: right; background-color: #cad7fe;"] {
                    @summary.total_lines
                }
                td[style = "text-align: right; background-color: #cad7fe;"] {
                    @summary.hit_lines
                }
            }
            tr {
                th[style = "text-align: right;"] {
                    "Functions"
                }
                @if (summary.hit_functions as f32 / summary.total_functions as f32)
                    * 100.0 >= 90.0 {
                    td[style = "text-align: right; background-color: #a7fc9d;"] {
                        @format!(
                            "{:.02}%",
                            (summary.hit_functions as f32 / summary.total_functions as f32)
                            * 100.0
                        )
                    }
                } else if (summary.hit_functions as f32 / summary.total_functions as f32)
                    * 100.0 >= 30.0 {
                    td[style = "text-align: right; background-color: #ffea20;"] {
                        @format!(
                            "{:.02}%",
                            (summary.hit_functions as f32 / summary.total_functions as f32)
                            * 100.0
                        )
                    }

                } else {
                    td[style = "text-align: right; background-color: #ff6230;"] {
                        @format!(
                            "{:.02}%",
                            (summary.hit_functions as f32 / summary.total_functions as f32)
                            * 100.0
                        )
                    }
                }
                td[style = "text-align: right; background-color: #cad7fe;"] {
                    @summary.total_functions
                }
                td[style = "text-align: right; background-color: #cad7fe;"] {
                    @summary.hit_functions
                }
            }
        }
    }

    Listing(
        lines: Vec<HtmlLineInfo>,
    ) {
        table[
            cellpadding = 0,
            cellspacing = 0,
            border = 0,
            style = "font-family: monospace, monospace; border-collapse: separate; border-spacing: 1em 0;"
        ] {
            tbody {
                tr {
                    th[style = "text-align: right;"] {
                        "Line"
                    }
                    th[style = "text-align: right;"] {
                        "Hits"
                    }
                    th[style = "text-align: left;"] {
                        "Source Code"
                    }
                }
                @for (i, line_info) in lines.iter().enumerate() {
                    tr {
                        td[style = "text-align: right;"] {
                            @{i + 1}
                        }
                        td[style = "text-align: right;"] {
                            @if let Some(hit_count) = line_info.hit_count {
                                @hit_count
                            } else {
                                "-"
                            }
                        }
                        @if let Some(hit_count) = line_info.hit_count {
                            @if hit_count == 0 {
                                td[style = "text-align: left; background-color: #ff6230;"] {
                                    @for _ in 0..line_info.leading_spaces {
                                        @markup::raw("&nbsp;")
                                    }
                                    @line_info.line
                                }
                            } else {
                                td[style = "text-align: left; background-color: #cad7fe;"] {
                                    @for _ in 0..line_info.leading_spaces {
                                        @markup::raw("&nbsp;")
                                    }
                                    @line_info.line
                                }
                            }
                        } else {
                            td[style = "text-align: left;"] {
                                @for _ in 0..line_info.leading_spaces {
                                    @markup::raw("&nbsp;")
                                }
                                @line_info.line
                            }
                        }
                    }
                }
            }
        }
    }

    FunctionListing(
        functions: Vec<HtmlFunctionInfo>,
    ) {
        table[
            cellpadding = 0,
            cellspacing = 0,
            border = 0,
            style = "font-family: monospace, monospace; border-collapse: separate; border-spacing: 1em 0;"
        ] {
            tbody {
                tr {
                    th[style = "text-align: right;"] {
                        "Function"
                    }
                    th[style = "text-align: right;"] {
                        "Hits"
                    }
                }
                @for function_info in functions.iter() {
                    tr {
                        @if let Some(hit_count) = function_info.hit_count {
                            @if hit_count == 0 {
                                td[style = "text-align: left; background-color: #ff6230;"] {
                                    @function_info.name
                                }
                            } else {
                                td[style = "text-align: left; background-color: #cad7fe;"] {
                                   @function_info.name
                                }
                            }
                        } else {
                            td[style = "text-align: left;"] {
                                @function_info.name
                            }
                        }
                        td[style = "text-align: right;"] {
                            @if let Some(hit_count) = function_info.hit_count {
                                @hit_count
                            } else {
                                "-"
                            }
                        }
                    }
                }
            }
        }
    }

    FilePage<L, FL>(listing: L, function_listing: FL) where L: Render, FL: Render {
        tr[style = "border-bottom: 1px solid black;"] {
            td {
                @listing
            }
        }
        tr[style = "border-bottom: 1px solid black;"] {
            td {
                @function_listing
            }
        }
    }

    DirectoryPage(summaries: Vec<HtmlSummaryInfo>) {
        tr[style = "border-bottom: 1px solid black;"] {
            table[width = "100%", style = "border-collapse: collapse;"] {
                tr[style = "border-bottom: 1px solid black;"] {
                    th {
                        "File/Directory"
                    }
                    th {
                        "Line Coverage"
                    }
                    th {
                        "Total Lines"
                    }
                    th {
                        "Hit Lines"
                    }
                    th {
                        "Function Coverage"
                    }
                    th {
                        "Total Functions"
                    }
                    th {
                        "Hit Functions"
                    }
                }
                @for summary in summaries {
                    tr[style = "border-bottom: 1px solid black;"] {
                        td {
                            @if summary.is_dir {
                                @if let Some(filename) = &summary.filename {
                                    a[href = PathBuf::from(filename).join("index.html").to_string_lossy().to_string()] {
                                        @summary.filename
                                    }
                                } else {
                                    "<unknown>"
                                }

                            } else if let Some(filename) = &summary.filename {
                                a[href = format!("{}.html", filename)] {
                                    @summary.filename
                                }
                            } else {
                                "<unknown>"
                            }
                        }
                        @if (summary.hit_lines as f32 / summary.total_lines as f32)
                            * 100.0 >= 90.0 {
                            td[style = "text-align: right; background-color: #a7fc9d;"] {
                                @format!(
                                    "{:.02}%",
                                    (summary.hit_lines as f32 / summary.total_lines as f32)
                                    * 100.0
                                )
                            }
                        } else if (summary.hit_lines as f32 / summary.total_lines as f32)
                            * 100.0 >= 30.0 {
                            td[style = "text-align: right; background-color: #ffea20;"] {
                                @format!(
                                    "{:.02}%",
                                    (summary.hit_lines as f32 / summary.total_lines as f32)
                                    * 100.0
                                )
                            }
                        } else {
                            td[style = "text-align: right; background-color: #ff6230;"] {
                                @format!(
                                    "{:.02}%",
                                    (summary.hit_lines as f32 / summary.total_lines as f32)
                                    * 100.0
                                )
                            }
                        }
                        td[style = "text-align: right; background-color: #cad7fe;"] {
                            @summary.total_lines
                        }
                        td[style = "text-align: right; background-color: #cad7fe;"] {
                            @summary.hit_lines
                        }
                        @if (summary.hit_functions as f32 / summary.total_functions as f32)
                            * 100.0 >= 90.0 {
                            td[style = "text-align: right; background-color: #a7fc9d;"] {
                                @format!(
                                    "{:.02}%",
                                    (summary.hit_functions as f32 / summary.total_functions as f32)
                                    * 100.0
                                )
                            }
                        } else if (summary.hit_functions as f32 / summary.total_functions as f32)
                            * 100.0 >= 30.0 {
                            td[style = "text-align: right; background-color: #ffea20;"] {
                                @format!(
                                    "{:.02}%",
                                    (summary.hit_functions as f32 / summary.total_functions as f32)
                                    * 100.0
                                )
                            }
                        } else {
                            td[style = "text-align: right; background-color: #ff6230;"] {
                                @format!(
                                    "{:.02}%",
                                    (summary.hit_functions as f32 / summary.total_functions as f32)
                                    * 100.0
                                )
                            }
                        }
                        td[style = "text-align: right; background-color: #cad7fe;"] {
                            @summary.total_functions
                        }
                        td[style = "text-align: right; background-color: #cad7fe;"] {
                            @summary.hit_functions
                        }
                    }
                }
            }
        }
    }


    Page<H, CV, S, M>(head: H, current_view: CV, summary: S, main: M) where H: Render, CV: Render, S: Render, M: Render  {
        @markup::doctype()
        html {
            head {
                @head
                @Style {}
                meta[charse = "utf-8"];
                title { "TSFFS Coverage Report" }
            }
            body {
                table[width = "100%", style = "border-collapse: collapse;"] {
                    tr[style = "text-align: center; border-bottom: 1px solid black;"] {
                        td {
                            "TSFFS Code Coverage Report"
                        }
                    }
                    tr[style = "border-bottom: 1px solid black;"] {
                        td {
                            @current_view
                        }
                        td {
                            @summary
                        }
                    }
                    @main
                }
            }
        }
    }

}
