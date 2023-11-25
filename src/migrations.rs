use nfl_schedule_simulator::execute;
use std::fs::File;
use std::io::prelude::*;

pub fn create() {
    execute_sql_file("migrations/up.sql");
}

pub fn destroy() {
    execute_sql_file("migrations/down.sql");
}

pub fn rebuild() {
    destroy();
    create();
}

fn execute_sql_file(file_path: &str) {
    let mut file = File::open(file_path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    let statements: Vec<String> = parse_sql(contents);

    for statement in statements {
        println!("{}", statement);
        execute(statement)
    }
}

fn parse_sql(raw_sql: String) -> Vec<String> {
    let mut statements = Vec::new();
    let mut buffer = String::new();
    for line in raw_sql.lines() {
        buffer += line;
        if line.contains(";") {
            statements.push(buffer.clone());
            buffer = String::new();
        }
    }

    statements
}
