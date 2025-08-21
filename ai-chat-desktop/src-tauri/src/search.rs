//! Full-text search module using Tantivy with custom Jieba tokenizer for Chinese text.

use crate::{ChatSession, get_app_data_dir};
use serde::{Deserialize, Serialize};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index, ReloadPolicy};
use tantivy::directory::MmapDirectory;
use tantivy::tokenizer::{Token, TokenStream, Tokenizer};
use jieba_rs::Jieba;
use std::sync::Arc;
use tracing::info;

const INDEX_DIR: &str = ".index";
const MEMORY_ARENA_NUM_BYTES: usize = 50_000_000; // 50MB

#[derive(Serialize, Deserialize, Debug)]
pub struct SearchResult {
    pub session_id: String,
    pub score: f32,
}

// Custom Jieba tokenizer implementation
#[derive(Clone)]
pub struct JiebaTokenizer {
    jieba: Arc<Jieba>,
}

impl JiebaTokenizer {
    pub fn new() -> Self {
        JiebaTokenizer {
            jieba: Arc::new(Jieba::new()),
        }
    }
}

impl Tokenizer for JiebaTokenizer {
    type TokenStream<'a> = JiebaTokenStream;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        let tokens = self.jieba.tokenize(text, jieba_rs::TokenizeMode::Default, true);
        let mut tantivy_tokens = Vec::new();
        
        for token in tokens {
            tantivy_tokens.push(Token {
                offset_from: token.start,
                offset_to: token.end,
                position: token.start,
                text: token.word.to_string(),
                position_length: token.end - token.start,
            });
        }
        
        JiebaTokenStream {
            tokens: tantivy_tokens,
            index: 0,
        }
    }
}

pub struct JiebaTokenStream {
    tokens: Vec<Token>,
    index: usize,
}

impl TokenStream for JiebaTokenStream {
    fn advance(&mut self) -> bool {
        if self.index < self.tokens.len() {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn token(&self) -> &Token {
        &self.tokens[self.index - 1]
    }

    fn token_mut(&mut self) -> &mut Token {
        &mut self.tokens[self.index - 1]
    }
}

pub struct Searcher {
    pub index: Index,
    schema: Schema,
}

impl Searcher {
    /// Creates or opens a Tantivy index in the app's data directory.
    pub fn new() -> Result<Self, String> {
        let index_path = get_app_data_dir().join(INDEX_DIR);
        if !index_path.exists() {
            std::fs::create_dir_all(&index_path).map_err(|e| e.to_string())?;
        }

        // Define text options with jieba tokenizer
        let text_options = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("jieba")
                    .set_index_option(IndexRecordOption::WithFreqsAndPositions)
            )
            .set_stored();

        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("session_id", STRING | STORED);
        schema_builder.add_text_field("title", text_options.clone());
        schema_builder.add_text_field("content", text_options);
        let schema = schema_builder.build();

        let dir = MmapDirectory::open(&index_path)
            .map_err(|e| format!("Failed to open directory: {}", e))?;
        let index = Index::open_or_create(dir, schema.clone())
            .map_err(|e| format!("Failed to open or create index: {}", e))?;

        // Register custom Jieba tokenizer for Chinese text
        index.tokenizers().register("jieba", JiebaTokenizer::new());

        Ok(Searcher { index, schema })
    }

    /// Rebuilds the entire search index from all chat sessions.
    pub fn rebuild_index(&self, sessions: Vec<ChatSession>) -> Result<usize, String> {
        info!("Starting to rebuild search index for {} sessions.", sessions.len());
        let mut index_writer = self.index.writer(MEMORY_ARENA_NUM_BYTES)
            .map_err(|e| e.to_string())?;

        // Clear existing documents
        index_writer.delete_all_documents().map_err(|e| e.to_string())?;

        let mut doc_count = 0;
        for session in sessions {
            // Index session title
            index_writer.add_document(doc!(
                self.schema.get_field("session_id").unwrap() => session.id.clone(),
                self.schema.get_field("title").unwrap() => session.title.clone(),
                self.schema.get_field("content").unwrap() => String::new() // Empty content for title doc
            )).map_err(|e| e.to_string())?;
            doc_count += 1;

            // Index each message
            for message in session.messages {
                // Only index user and assistant messages
                if message.role == "user" || message.role == "assistant" {
                    index_writer.add_document(doc!(
                        self.schema.get_field("session_id").unwrap() => session.id.clone(),
                        self.schema.get_field("title").unwrap() => session.title.clone(),
                        self.schema.get_field("content").unwrap() => message.content
                    )).map_err(|e| e.to_string())?;
                    doc_count += 1;
                }
            }
        }

        // Commit changes
        index_writer.commit().map_err(|e| e.to_string())?;
        info!("Successfully rebuilt index with {} documents.", doc_count);
        Ok(doc_count)
    }

    /// Searches the index for a given query string.
    pub fn search(&self, query_str: &str) -> Result<Vec<SearchResult>, String> {
        let reader = self.index.reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()
            .map_err(|e| e.to_string())?;

        let searcher = reader.searcher();
        let query_parser = QueryParser::for_index(&self.index, vec![
            self.schema.get_field("title").unwrap(),
            self.schema.get_field("content").unwrap(),
        ]);

        let query = query_parser.parse_query(query_str)
            .map_err(|e| format!("Failed to parse query: {}", e))?;

        let top_docs = searcher.search(&query, &TopDocs::with_limit(100))
            .map_err(|e| e.to_string())?;

        let mut results_map: std::collections::HashMap<String, f32> = std::collections::HashMap::new();

        for (score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc::<tantivy::TantivyDocument>(doc_address).map_err(|e| e.to_string())?;
            let session_id_field = self.schema.get_field("session_id").unwrap();
            
            if let Some(session_id_val) = retrieved_doc.get_first(session_id_field) {
                match session_id_val {
                    tantivy::schema::OwnedValue::Str(session_id) => {
                        // If the session is already in the map, add the scores.
                        // This gives more weight to sessions with more matches.
                        *results_map.entry(session_id.clone()).or_insert(0.0) += score;
                    }
                    _ => {} // Ignore non-string values
                }
            }
        }
        
        // Convert map to Vec and sort by score
        let mut final_results: Vec<SearchResult> = results_map.into_iter()
            .map(|(session_id, score)| SearchResult { session_id, score })
            .collect();
            
        final_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(final_results)
    }

    /// Adds or updates a session in the search index.
    pub fn add_or_update_session(&self, session: &ChatSession) -> Result<(), String> {
        let mut index_writer: tantivy::IndexWriter<tantivy::TantivyDocument> = self.index.writer(MEMORY_ARENA_NUM_BYTES)
            .map_err(|e| e.to_string())?;

        // First, delete any existing documents for this session
        let session_id_field = self.schema.get_field("session_id").unwrap();
        let term = tantivy::Term::from_field_text(session_id_field, &session.id);
        index_writer.delete_term(term);

        // Then add the new documents
        // Index session title
        index_writer.add_document(doc!(
            self.schema.get_field("session_id").unwrap() => session.id.clone(),
            self.schema.get_field("title").unwrap() => session.title.clone(),
            self.schema.get_field("content").unwrap() => String::new() // Empty content for title doc
        )).map_err(|e| e.to_string())?;

        // Index each message
        for message in &session.messages {
            // Only index user and assistant messages
            if message.role == "user" || message.role == "assistant" {
                index_writer.add_document(doc!(
                    self.schema.get_field("session_id").unwrap() => session.id.clone(),
                    self.schema.get_field("title").unwrap() => session.title.clone(),
                    self.schema.get_field("content").unwrap() => message.content.clone()
                )).map_err(|e| e.to_string())?;
            }
        }

        // Commit changes
        index_writer.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Removes a session from the search index.
    pub fn remove_session(&self, session_id: &str) -> Result<(), String> {
        let mut index_writer: tantivy::IndexWriter<tantivy::TantivyDocument> = self.index.writer(MEMORY_ARENA_NUM_BYTES)
            .map_err(|e| e.to_string())?;

        // Delete all documents for this session
        let session_id_field = self.schema.get_field("session_id").unwrap();
        let term = tantivy::Term::from_field_text(session_id_field, session_id);
        index_writer.delete_term(term);

        // Commit changes
        index_writer.commit().map_err(|e| e.to_string())?;
        Ok(())
    }
}