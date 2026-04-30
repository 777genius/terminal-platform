pub mod connection;
pub mod executor;
pub mod migrations;
pub mod schema;

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    const MIGRATION_SQL: &str =
        include_str!("../../migrations/00000000000001_terminal_persistence_v2/up.sql");
    const SCHEMA_RS: &str = include_str!("schema.rs");
    const MAX_DIESEL_TABLE_COLUMNS: usize = 32;

    #[test]
    fn diesel_schema_tables_match_migration_tables() {
        let migration_tables = migration_table_names();
        let schema_tables = diesel_schema_table_names();

        assert_eq!(schema_tables, migration_tables);
    }

    #[test]
    fn diesel_table_column_budget_stays_bounded() {
        let tables = diesel_schema_tables_with_column_counts();
        let (table_name, column_count) =
            tables.iter().max_by_key(|(_, column_count)| *column_count).unwrap();

        assert!(
            *column_count <= MAX_DIESEL_TABLE_COLUMNS,
            "table `{table_name}` has {column_count} columns, budget is {MAX_DIESEL_TABLE_COLUMNS}"
        );
    }

    fn migration_table_names() -> BTreeSet<String> {
        MIGRATION_SQL
            .lines()
            .filter_map(|line| {
                line.trim()
                    .strip_prefix("CREATE TABLE IF NOT EXISTS ")
                    .and_then(|value| value.split_whitespace().next())
                    .map(|value| value.trim_end_matches('(').to_string())
            })
            .collect()
    }

    fn diesel_schema_table_names() -> BTreeSet<String> {
        diesel_schema_tables_with_column_counts()
            .into_iter()
            .map(|(table_name, _)| table_name)
            .collect()
    }

    fn diesel_schema_tables_with_column_counts() -> Vec<(String, usize)> {
        let mut tables = Vec::new();
        let mut lines = SCHEMA_RS.lines().peekable();
        while let Some(line) = lines.next() {
            if line.trim() != "diesel::table! {" {
                continue;
            }
            let Some(header) = lines.next() else {
                break;
            };
            let Some(table_name) = header.split_whitespace().next() else {
                continue;
            };
            let mut column_count = 0;
            for line in lines.by_ref() {
                let line = line.trim();
                if line == "}" {
                    break;
                }
                if line.contains(" -> ") {
                    column_count += 1;
                }
            }
            tables.push((table_name.to_string(), column_count));
        }
        tables
    }
}
