//! The rows of the sidebar's schema tree.

use std::collections::BTreeSet;

use quarry_engine::schema::{ColumnInfo, Schema, Table, TableKind};

/// One visible row of the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeRow {
    /// A table or view; `expanded` when its columns are listed below it.
    Table {
        /// Table name.
        name: String,
        /// What the row's trailing text says: the column count, or "view".
        detail: String,
        /// Whether the table's columns are shown.
        expanded: bool,
    },
    /// A column of the expanded table above it.
    Column {
        /// Column name.
        name: String,
        /// Declared type, with `· PK` for primary-key columns.
        detail: String,
    },
}

/// The rows to show, given which tables are expanded.
pub fn rows(schema: &Schema, expanded: &BTreeSet<String>) -> Vec<TreeRow> {
    let mut rows = Vec::new();
    for table in &schema.tables {
        let open = expanded.contains(&table.name);
        rows.push(table_row(table, open));
        if open {
            rows.extend(table.columns.iter().map(column_row));
        }
    }
    rows
}

/// Expands `name` when collapsed and collapses it when expanded.
pub fn toggle(expanded: &mut BTreeSet<String>, name: &str) {
    if !expanded.remove(name) {
        expanded.insert(name.to_owned());
    }
}

fn table_row(table: &Table, expanded: bool) -> TreeRow {
    let detail = match table.kind {
        TableKind::View => "view".to_owned(),
        TableKind::Table => table.columns.len().to_string(),
    };
    TreeRow::Table {
        name: table.name.clone(),
        detail,
        expanded,
    }
}

fn column_row(column: &ColumnInfo) -> TreeRow {
    let type_name = if column.type_name.is_empty() {
        "any"
    } else {
        column.type_name.as_str()
    };
    let detail = if column.primary_key {
        format!("{type_name} · PK")
    } else {
        type_name.to_owned()
    };
    TreeRow::Column {
        name: column.name.clone(),
        detail,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use quarry_engine::schema::{ColumnInfo, Schema, Table, TableKind};

    use super::{TreeRow, rows, toggle};

    fn column(name: &str, type_name: &str, primary_key: bool) -> ColumnInfo {
        ColumnInfo {
            name: name.to_owned(),
            type_name: type_name.to_owned(),
            primary_key,
        }
    }

    fn schema() -> Schema {
        Schema {
            tables: vec![
                Table {
                    name: "missions".to_owned(),
                    kind: TableKind::Table,
                    columns: vec![column("id", "INTEGER", true), column("name", "TEXT", false)],
                },
                Table {
                    name: "roster".to_owned(),
                    kind: TableKind::View,
                    columns: vec![column("who", "", false)],
                },
            ],
        }
    }

    fn table(name: &str, detail: &str, expanded: bool) -> TreeRow {
        TreeRow::Table {
            name: name.to_owned(),
            detail: detail.to_owned(),
            expanded,
        }
    }

    fn col(name: &str, detail: &str) -> TreeRow {
        TreeRow::Column {
            name: name.to_owned(),
            detail: detail.to_owned(),
        }
    }

    #[test]
    fn collapsed_tables_show_their_column_count_or_view() {
        assert_eq!(
            rows(&schema(), &BTreeSet::new()),
            [
                table("missions", "2", false),
                table("roster", "view", false)
            ]
        );
    }

    #[test]
    fn an_expanded_table_lists_its_columns_with_types() {
        let expanded = BTreeSet::from(["missions".to_owned(), "roster".to_owned()]);
        assert_eq!(
            rows(&schema(), &expanded),
            [
                table("missions", "2", true),
                col("id", "INTEGER · PK"),
                col("name", "TEXT"),
                table("roster", "view", true),
                col("who", "any"),
            ]
        );
    }

    #[test]
    fn toggling_twice_collapses_again() {
        let mut expanded = BTreeSet::new();
        toggle(&mut expanded, "missions");
        assert!(expanded.contains("missions"));
        toggle(&mut expanded, "missions");
        assert!(expanded.is_empty());
    }
}
