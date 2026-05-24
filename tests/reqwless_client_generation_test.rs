use openapi_to_rust::{CodeGenerator, GeneratorConfig, analysis::SchemaAnalyzer};
use serde_json::json;
use std::path::PathBuf;

fn create_test_config() -> GeneratorConfig {
    GeneratorConfig {
        spec_path: PathBuf::from("test.json"),
        output_dir: PathBuf::from("test_output"),
        module_name: "test".to_string(),
        enable_async_client: false,
        enable_reqwless_client: true,
        ..Default::default()
    }
}

fn create_minimal_spec() -> serde_json::Value {
    json!({
        "openapi": "3.0.0",
        "info": { "title": "Test API", "version": "1.0.0" },
        "paths": {
            "/users": {
                "get": {
                    "operationId": "listUsers",
                    "parameters": [
                        {
                            "name": "limit",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "integer" }
                        },
                        {
                            "name": "x-request-id",
                            "in": "header",
                            "required": false,
                            "schema": { "type": "string" }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Success",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": { "$ref": "#/components/schemas/User" }
                                    }
                                }
                            }
                        }
                    }
                },
                "post": {
                    "operationId": "createUser",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/CreateUserRequest" }
                            }
                        }
                    },
                    "responses": {
                        "201": {
                            "description": "Created",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/User" }
                                }
                            }
                        },
                        "400": {
                            "description": "Bad request",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/ErrorResponse" }
                                }
                            }
                        }
                    }
                }
            },
            "/users/{id}": {
                "delete": {
                    "operationId": "deleteUser",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string" }
                        }
                    ],
                    "responses": {
                        "204": { "description": "No Content" }
                    }
                }
            }
        },
        "components": {
            "schemas": {
                "User": {
                    "type": "object",
                    "required": ["id", "name"],
                    "properties": {
                        "id": { "type": "string" },
                        "name": { "type": "string" }
                    }
                },
                "CreateUserRequest": {
                    "type": "object",
                    "required": ["name"],
                    "properties": {
                        "name": { "type": "string" }
                    }
                },
                "ErrorResponse": {
                    "type": "object",
                    "required": ["message"],
                    "properties": {
                        "message": { "type": "string" }
                    }
                }
            }
        }
    })
}

#[test]
fn test_reqwless_client_struct_generation() {
    let generator = CodeGenerator::new(create_test_config());
    let client_code = generator.generate_reqwless_client_struct();
    let syntax_tree = syn::parse2::<syn::File>(client_code).expect("Failed to parse generated code");
    let code_str = prettyplease::unparse(&syntax_tree);

    assert!(code_str.contains("pub struct EmbeddedHttpClient"));
    assert!(code_str.contains("embedded_nal_async::TcpConnect"));
    assert!(code_str.contains("embedded_nal_async::Dns"));
    assert!(code_str.contains("ReqwlessHttpClient"));
    assert!(code_str.contains("rx_buf"));
    assert!(code_str.contains("custom_headers"));
    assert!(code_str.contains("BTreeMap<String, String>"));
}

#[test]
fn test_reqwless_constructor_and_builders() {
    let generator = CodeGenerator::new(create_test_config());
    let client_code = generator.generate_reqwless_client_struct();
    let syntax_tree = syn::parse2::<syn::File>(client_code).expect("Failed to parse generated code");
    let code_str = prettyplease::unparse(&syntax_tree);

    assert!(code_str.contains("pub fn new"));
    assert!(code_str.contains("pub fn with_rx_buf_size"));
    assert!(code_str.contains("DEFAULT_RX_BUF_SIZE"));
    assert!(code_str.contains("pub fn with_base_url"));
    assert!(code_str.contains("pub fn with_api_key"));
    assert!(code_str.contains("pub fn with_header"));
    assert!(code_str.contains("pub fn with_headers"));
}

#[test]
fn test_full_reqwless_client_generation() {
    let spec = create_minimal_spec();
    let mut analyzer = SchemaAnalyzer::new(spec).expect("Failed to create analyzer");
    let analysis = analyzer.analyze().expect("Failed to analyze spec");
    let generator = CodeGenerator::new(create_test_config());

    let client_code = generator
        .generate_reqwless_client(&analysis)
        .expect("Failed to generate reqwless client");

    assert!(client_code.contains("extern crate alloc"));
    assert!(client_code.contains("use embedded_io_async::Read"));
    assert!(client_code.contains("use reqwless::client::HttpClient as ReqwlessHttpClient"));
    assert!(client_code.contains("pub enum HttpError"));
    assert!(client_code.contains("pub struct ApiError"));
    assert!(client_code.contains("pub enum ApiOpError"));
    assert!(client_code.contains("pub struct EmbeddedHttpClient"));
    assert!(client_code.contains("pub async fn list_users"));
    assert!(client_code.contains("pub async fn create_user"));
    assert!(client_code.contains("pub async fn delete_user"));
    assert!(client_code.contains("Method::GET"));
    assert!(client_code.contains("Method::POST"));
    assert!(client_code.contains("Method::DELETE"));
}

#[test]
fn test_reqwless_client_handles_auth_headers_and_query_params() {
    let spec = create_minimal_spec();
    let mut analyzer = SchemaAnalyzer::new(spec).expect("Failed to create analyzer");
    let analysis = analyzer.analyze().expect("Failed to analyze spec");
    let generator = CodeGenerator::new(create_test_config());

    let client_code = generator
        .generate_reqwless_client(&analysis)
        .expect("Failed to generate reqwless client");

    assert!(client_code.contains("__pct_encode_path_segment"));
    assert!(client_code.contains("__pct_encode_query_component"));
    assert!(client_code.contains("request_url.push(separator)"));
    assert!(client_code.contains("Authorization"));
    assert!(client_code.contains("x-request-id"));
    assert!(client_code.contains("header_storage"));
}

#[test]
fn test_reqwless_client_handles_json_bodies_and_typed_errors() {
    let spec = create_minimal_spec();
    let mut analyzer = SchemaAnalyzer::new(spec).expect("Failed to create analyzer");
    let analysis = analyzer.analyze().expect("Failed to analyze spec");
    let generator = CodeGenerator::new(create_test_config());

    let client_code = generator
        .generate_reqwless_client(&analysis)
        .expect("Failed to generate reqwless client");

    assert!(client_code.contains("serde_json::to_vec(&request)"));
    assert!(client_code.contains("ContentType::ApplicationJson"));
    assert!(client_code.contains("pub enum CreateUserApiError"));
    assert!(client_code.contains("Status400(ErrorResponse)"));
    assert!(client_code.contains("serde_json::from_slice::<ErrorResponse>"));
    assert!(client_code.contains("ApiError"));
}

#[test]
fn test_reqwless_client_generated_code_parses() {
    let spec = create_minimal_spec();
    let mut analyzer = SchemaAnalyzer::new(spec).expect("Failed to create analyzer");
    let analysis = analyzer.analyze().expect("Failed to analyze spec");
    let generator = CodeGenerator::new(create_test_config());

    let client_code = generator
        .generate_reqwless_client(&analysis)
        .expect("Failed to generate reqwless client");

    syn::parse_file(&client_code).expect("Generated reqwless client should parse");
}

#[test]
fn test_generate_all_includes_reqwless_client_when_enabled() {
    let spec = create_minimal_spec();
    let mut analyzer = SchemaAnalyzer::new(spec).expect("Failed to create analyzer");
    let mut analysis = analyzer.analyze().expect("Failed to analyze spec");
    let generator = CodeGenerator::new(create_test_config());

    let result = generator
        .generate_all(&mut analysis)
        .expect("Failed to generate all files");

    assert!(
        result
            .files
            .iter()
            .any(|file| file.path == PathBuf::from("types.rs"))
    );
    assert!(
        result
            .files
            .iter()
            .any(|file| file.path == PathBuf::from("reqwless_client.rs"))
    );
    assert!(
        !result
            .files
            .iter()
            .any(|file| file.path == PathBuf::from("client.rs"))
    );
}
