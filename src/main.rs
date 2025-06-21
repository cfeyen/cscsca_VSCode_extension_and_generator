use std::{
    fs, io::{self, Write}, process::Command
};

use serde::{Deserialize, Serialize};

fn main() {
    if let Err(e) = gen_ext() {
        println!("Error: {e}")
    }
}

fn gen_ext() -> io::Result<()> {
    let source_dir = get("Enter source directory");
    let target_dir = get("Enter target directory name");

    Command::new("cp")
        .arg("-r")
        .arg(&source_dir)
        .arg(&target_dir)
        .output()?;

    let gen_file = get("Enter JSON gen file");
    let gen_json = fs::read_to_string(gen_file)?;
    let gen_json = serde_json::from_str::<GrammarGenerator>(&gen_json)?;

    let grammar: GrammarFile = gen_json.into();
    let grammar = serde_json::to_string(&grammar)?;

    fs::write(format!("{target_dir}/syntaxes/grammar.json"), grammar)
}


fn get(prompt: &str) -> String {
    print!("{prompt}: ");
    _ = io::stdout().flush();
    let mut buffer = String::new();
    _ = io::stdin().read_line(&mut buffer);
    buffer.trim().to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct GrammarGenerator {
    breaking_ahead_placeholder: String,
    breaking_behind_placeholder: String,
    word_placeholder: String,
    breaking_chars: Vec<BreakingChar>,
    comments: Vec<ExpressionFormat>,
    expressions: Vec<ExpressionFormat>
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct BreakingChar {
    char: String,
    escapable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct ExpressionFormat {
    name: String,
    token_type: String,
    r#match: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct GrammarFile {
    name: String,
    patterns: Vec<Include>,
    repository: Repository,
    scopeName: String,
}

impl GrammarGenerator {
    fn expand(&self, s: &str) -> String {
        let non_breaking = format!("[^{}]", self.breaking_chars.iter()
            .map(|BreakingChar { char, escapable: _ }| char.to_string())
            .collect::<String>());

        let escaped_breaking = self.breaking_chars.iter()
            .flat_map(|BreakingChar { char, escapable }| if *escapable {
                Some(format!("\\\\{char}"))
            } else {
                None
            })
            .collect::<Vec<_>>()
            .join("|");

        let word = format!("({escaped_breaking}|{non_breaking})++");

        let breaking = self.breaking_chars.iter()
            .map(|c| c.char.clone())
            .collect::<Vec<_>>()
            .join("|");

        s.replace(&self.word_placeholder, &word)
            .replace(&self.breaking_behind_placeholder, &format!("(?<={breaking})"))
            .replace(&self.breaking_ahead_placeholder, &format!("(?={breaking})"))
    }
}

impl From<GrammarGenerator> for GrammarFile {
    fn from(grammar_gen: GrammarGenerator) -> Self {
        let GrammarGenerator {
            breaking_ahead_placeholder: _,
            breaking_behind_placeholder: _,
            word_placeholder: _,
            breaking_chars: _,
            comments,
            expressions,
        } = &grammar_gen;

        Self {
            name: "CSCSCA".to_string(),
            patterns: vec![Include("comment".to_string()), Include("expression".to_string())],
            repository: Repository {
                comment: Patterns {
                    patterns: comments.into_iter().
                        map(|ef| Include(ef.name.clone()))
                        .collect()
                },
                expression: Patterns {
                    patterns: expressions.into_iter().
                        map(|ef| Include(ef.name.clone()))
                        .collect()
                },
                expr_formats: comments.into_iter()
                    .chain(expressions.into_iter())
                    .map(|ExpressionFormat { name, token_type, r#match }| ExpressionFormat {
                        name: name.clone(),
                        token_type: token_type.clone(),
                        r#match: grammar_gen.expand(r#match),
                    })
                    .collect(),
            },
            scopeName: "source.sca".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Include(String);

impl Serialize for Include {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        let mut map = serde_json::Map::new();
        map.insert("include".to_string(), format!("#{}", self.0).into());
        map.serialize(serializer)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Repository {
    comment: Patterns,
    expression: Patterns,
    expr_formats: Vec<ExpressionFormat>
}

impl Serialize for Repository {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        let mut map = serde_json::Map::new();
        map.insert("comment".to_string(), serde_json::to_value(&self.comment).unwrap());
        map.insert("expression".to_string(), serde_json::to_value(&self.expression).unwrap());
        self.expr_formats.iter()
            .for_each(|ExpressionFormat { name, token_type, r#match }| {
                map.insert(name.clone(), serde_json::to_value(Expression {
                    name: token_type.clone(),
                    r#match: r#match.clone(),
                }).unwrap());
            });
        map.serialize(serializer)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Expression {
    name: String,
    r#match: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Patterns {
    patterns: Vec<Include>
}