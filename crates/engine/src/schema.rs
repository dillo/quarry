//! What a database contains, for the schema tree.

/// Every table and view in a database, sorted by name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schema {
    /// Tables and views.
    pub tables: Vec<Table>,
}

/// Whether an entry is a stored table or a view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableKind {
    /// A stored table.
    Table,
    /// A view.
    View,
}

/// One table or view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// Table name.
    pub name: String,
    /// Table or view.
    pub kind: TableKind,
    /// Columns in declaration order.
    pub columns: Vec<ColumnInfo>,
}

/// One column of a table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnInfo {
    /// Column name.
    pub name: String,
    /// Declared type (may be empty).
    pub type_name: String,
    /// Whether the column is part of the primary key.
    pub primary_key: bool,
}
