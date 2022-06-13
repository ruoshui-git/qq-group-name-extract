// Copyright 2019 Mitchell Kember. Subject to the MIT License.

//! Utility for extracting data from HTML tables.
//!
//! This library allows you to parse tables from HTML documents and iterate over
//! their rows. There are three entry points:
//!
//! - [`Table::find_first`] finds the first table.
//! - [`Table::find_by_id`] finds a table by its HTML id.
//! - [`Table::find_by_headers`] finds a table that has certain headers.
//!
//! Each of these returns an `Option<`[`Table`]`>`, since there might not be any
//! matching table in the HTML. Once you have a table, you can iterate over it
//! and access the contents of each [`Row`].
//!
//! # Examples
//!
//! Here is a simple example that uses [`Table::find_first`] to print the cells
//! in each row of a table:
//!
//! ```
//! let html = r#"
//!     <table>
//!         <tr><th>Name</th><th>Age</th></tr>
//!         <tr><td>John</td><td>20</td></tr>
//!     </table>
//! "#;
//! let table = table_extract::Table::find_first(html).unwrap();
//! for row in &table {
//!     println!(
//!         "{} is {} years old",
//!         row.get("Name").unwrap_or("<name missing>"),
//!         row.get("Age").unwrap_or("<age missing>")
//!     )
//! }
//! ```
//!
//! If the document has multiple tables, we can use [`Table::find_by_headers`]
//! to identify the one we want:
//!
//! ```
//! let html = r#"
//!     <table></table>
//!     <table>
//!         <tr><th>Name</th><th>Age</th></tr>
//!         <tr><td>John</td><td>20</td></tr>
//!     </table>
//! "#;
//! let table = table_extract::Table::find_by_headers(html, &["Age"]).unwrap();
//! for row in &table {
//!     for cell in row {
//!         println!("Table cell: {}", cell);
//!     }
//! }
//! ```
//!
//! [`Table`]: struct.Table.html
//! [`Row`]: struct.Row.html
//! [`Table::find_first`]: struct.Table.html#method.find_first
//! [`Table::find_by_id`]: struct.Table.html#method.find_by_id
//! [`Table::find_by_headers`]: struct.Table.html#method.find_by_headers

use scraper::element_ref::ElementRef;
use scraper::{Html, Selector};
use std::collections::HashMap;

/// A map from `<th>` table headers to their zero-based positions.
///
/// For example, consider the following table:
///
/// ```html
/// <table>
///     <tr><th>Name</th><th>Age</th></tr>
///     <tr><td>John</td><td>20</td></tr>
/// </table>
/// ```
///
/// The `Headers` for this table would map "Name" to 0 and "Age" to 1.
pub type Headers = HashMap<String, usize>;

/// A parsed HTML table.
///
/// See [the module level documentation](index.html) for more.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Table {
    headers: Headers,
    data: Vec<Vec<String>>,
}

impl Table {
    /// Finds the first table in `html`.
    pub fn find_first(html: &str) -> Option<Table> {
        let html = Html::parse_fragment(html);
        html.select(&css("table")).next().map(Table::new)
    }

    /// Finds the table in `html` with an id of `id`.
    pub fn find_by_id(html: &str, id: &str) -> Option<Table> {
        let html = Html::parse_fragment(html);
        let selector = format!("table#{}", id);
        Selector::parse(&selector)
            .ok()
            .as_ref()
            .map(|s| html.select(s))
            .and_then(|mut s| s.next())
            .map(Table::new)
    }

    /// Finds the table in `html` whose first row contains all of the headers
    /// specified in `headers`. The order does not matter.
    ///
    /// If `headers` is empty, this is the same as
    /// [`find_first`](#method.find_first).
    pub fn find_by_headers<T>(html: &str, headers: &[T]) -> Option<Table>
    where
        T: AsRef<str>,
    {
        if headers.is_empty() {
            return Table::find_first(html);
        }

        let sel_table = css("table");
        let sel_tr = css("tr");
        let sel_th = css("th");

        let html = Html::parse_fragment(html);
        html.select(&sel_table)
            .find(|table| {
                table.select(&sel_tr).next().map_or(false, |tr| {
                    let cells = select_cells(tr, &sel_th);
                    headers.iter().all(|h| contains_str(&cells, h.as_ref()))
                })
            })
            .map(Table::new)
    }

    /// Returns the headers of the table.
    ///
    /// This will be empty if the table had no `<th>` tags in its first row. See
    /// [`Headers`](type.Headers.html) for more.
    pub fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Returns an iterator over the [`Row`](struct.Row.html)s of the table.
    ///
    /// Only `<td>` cells are considered when generating rows. If the first row
    /// of the table is a header row, meaning it contains at least one `<th>`
    /// cell, the iterator will start on the second row. Use
    /// [`headers`](#method.headers) to access the header row in that case.
    pub fn iter(&self) -> Iter {
        Iter {
            headers: &self.headers,
            iter: self.data.iter(),
        }
    }

    pub fn new(element: ElementRef) -> Table {
        let sel_tr = css("tr");
        let sel_th = css("th");
        let sel_td = css("td");

        let mut headers = HashMap::new();
        let mut rows = element.select(&sel_tr).peekable();
        if let Some(tr) = rows.peek() {
            for (i, th) in tr.select(&sel_th).enumerate() {
                headers.insert(cell_content(th), i);
            }
        }
        if !headers.is_empty() {
            rows.next();
        }
        let data = rows.map(|tr| select_cells(tr, &sel_td)).collect();

        Table { headers, data }
    }
}

impl<'a> IntoIterator for &'a Table {
    type Item = Row<'a>;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over the rows in a [`Table`](struct.Table.html).
pub struct Iter<'a> {
    headers: &'a Headers,
    iter: std::slice::Iter<'a, Vec<String>>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Row<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let headers = self.headers;
        self.iter.next().map(|cells| Row { headers, cells })
    }
}

/// A row in a [`Table`](struct.Table.html).
///
/// A row consists of a number of data cells stored as strings. If the row
/// contains the same number of cells as the table's header row, its cells can
/// be safely accessed by header names using [`get`](#method.get). Otherwise,
/// the data should be accessed via [`as_slice`](#method.as_slice) or by
/// iterating over the row.
///
/// This struct can be thought of as a lightweight reference into a table. As
/// such, it implements the `Copy` trait.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Row<'a> {
    headers: &'a Headers,
    cells: &'a [String],
}

impl<'a> Row<'a> {
    /// Returns the number of cells in the row.
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Returns `true` if the row contains no cells.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Returns the cell underneath `header`.
    ///
    /// Returns `None` if there is no such header, or if there is no cell at
    /// that position in the row.
    pub fn get(&self, header: &str) -> Option<&'a str> {
        self.headers
            .get(header)
            .and_then(|&i| self.cells.get(i).map(String::as_str))
    }

    /// Returns a slice containing all the cells.
    pub fn as_slice(&self) -> &'a [String] {
        self.cells
    }

    /// Returns an iterator over the cells of the row.
    pub fn iter(&self) -> std::slice::Iter<String> {
        self.cells.iter()
    }
}

impl<'a> IntoIterator for Row<'a> {
    type Item = &'a String;
    type IntoIter = std::slice::Iter<'a, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.cells.iter()
    }
}

fn css(selector: &'static str) -> Selector {
    Selector::parse(selector).unwrap()
}

fn select_cells(element: ElementRef, selector: &Selector) -> Vec<String> {
    element.select(selector).map(cell_content).collect()
}

fn cell_content(element: ElementRef) -> String {
    element.inner_html().trim().to_string()
}

fn contains_str(slice: &[String], item: &str) -> bool {
    slice.iter().any(|s| s == item)
}
