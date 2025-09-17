//! Scripting Engine Support
//!
//! Provides scripting engine implementations for Lua and other embedded languages.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

/// Scripting engine trait
#[async_trait::async_trait]
pub trait ScriptingEngine: Send + Sync {
    /// Engine name
    fn name(&self) -> &str;
    
    /// Supported script language
    fn language(&self) -> &str;
    
    /// Execute script code
    async fn execute(&self, code: &str, context: ScriptContext) -> Result<ScriptResult>;
    
    /// Execute script file
    async fn execute_file(&self, file_path: &str, context: ScriptContext) -> Result<ScriptResult>;
    
    /// Validate script syntax
    async fn validate_syntax(&self, code: &str) -> Result<SyntaxValidation>;
    
    /// Get engine capabilities
    fn capabilities(&self) -> EngineCapabilities;
}

/// Script execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptContext {
    /// Global variables
    pub globals: HashMap<String, serde_json::Value>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Execution timeout in seconds
    pub timeout_secs: Option<u64>,
    /// Maximum memory usage in bytes
    pub max_memory_bytes: Option<u64>,
    /// Allowed modules/libraries
    pub allowed_modules: Vec<String>,
    /// Security restrictions
    pub security: SecurityRestrictions,
}

/// Security restrictions for script execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRestrictions {
    /// Allow file system access
    pub allow_file_access: bool,
    /// Allow network access
    pub allow_network_access: bool,
    /// Allow system calls
    pub allow_system_calls: bool,
    /// Allowed file paths (if file access is enabled)
    pub allowed_paths: Vec<String>,
    /// Allowed network hosts (if network access is enabled)
    pub allowed_hosts: Vec<String>,
}

/// Script execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptResult {
    /// Execution success
    pub success: bool,
    /// Return value
    pub return_value: Option<serde_json::Value>,
    /// Output messages
    pub output: Vec<String>,
    /// Error messages
    pub errors: Vec<String>,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Memory usage in bytes
    pub memory_usage_bytes: Option<u64>,
    /// Script statistics
    pub statistics: ScriptStatistics,
}

/// Script execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptStatistics {
    /// Lines executed
    pub lines_executed: u32,
    /// Function calls made
    pub function_calls: u32,
    /// Variables created
    pub variables_created: u32,
    /// Peak memory usage
    pub peak_memory_bytes: Option<u64>,
}

/// Syntax validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxValidation {
    /// Syntax is valid
    pub is_valid: bool,
    /// Syntax errors
    pub errors: Vec<SyntaxError>,
    /// Syntax warnings
    pub warnings: Vec<SyntaxWarning>,
}

/// Syntax error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxError {
    /// Error message
    pub message: String,
    /// Line number
    pub line: u32,
    /// Column number
    pub column: u32,
    /// Error type
    pub error_type: String,
}

/// Syntax warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxWarning {
    /// Warning message
    pub message: String,
    /// Line number
    pub line: u32,
    /// Column number
    pub column: u32,
    /// Warning type
    pub warning_type: String,
}

/// Engine capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineCapabilities {
    /// Supports sandboxing
    pub supports_sandboxing: bool,
    /// Supports debugging
    pub supports_debugging: bool,
    /// Supports profiling
    pub supports_profiling: bool,
    /// Supports bytecode compilation
    pub supports_bytecode: bool,
    /// Maximum execution time supported
    pub max_execution_time_secs: Option<u64>,
    /// Maximum memory usage supported
    pub max_memory_bytes: Option<u64>,
    /// Supported modules/libraries
    pub supported_modules: Vec<String>,
}

/// Lua scripting engine implementation
pub struct LuaEngine {
    /// Engine configuration
    config: LuaEngineConfig,
}

/// Lua engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuaEngineConfig {
    /// Enable standard library
    pub enable_std_lib: bool,
    /// Enable debug library
    pub enable_debug: bool,
    /// Enable IO library
    pub enable_io: bool,
    /// Enable OS library
    pub enable_os: bool,
    /// Custom modules to load
    pub custom_modules: Vec<String>,
    /// Memory limit in bytes
    pub memory_limit: Option<u64>,
}

impl Default for ScriptContext {
    fn default() -> Self {
        Self {
            globals: HashMap::new(),
            environment: HashMap::new(),
            timeout_secs: Some(30),
            max_memory_bytes: Some(64 * 1024 * 1024), // 64MB
            allowed_modules: vec![],
            security: SecurityRestrictions {
                allow_file_access: false,
                allow_network_access: false,
                allow_system_calls: false,
                allowed_paths: vec![],
                allowed_hosts: vec![],
            },
        }
    }
}

impl Default for LuaEngineConfig {
    fn default() -> Self {
        Self {
            enable_std_lib: true,
            enable_debug: false,
            enable_io: false,
            enable_os: false,
            custom_modules: vec![],
            memory_limit: Some(64 * 1024 * 1024), // 64MB
        }
    }
}

impl LuaEngine {
    /// Create a new Lua engine
    pub fn new(config: LuaEngineConfig) -> Self {
        Self { config }
    }
    
    /// Create Lua instance with configuration
    fn create_lua(&self, context: &ScriptContext) -> Result<mlua::Lua> {
        let lua = mlua::Lua::new();
        
        // Set up security restrictions
        if !context.security.allow_file_access {
            // Disable file access functions
            lua.globals().set("io", mlua::Nil)?;
            lua.globals().set("loadfile", mlua::Nil)?;
            lua.globals().set("dofile", mlua::Nil)?;
        }
        
        if !context.security.allow_system_calls {
            // Disable system access
            lua.globals().set("os", mlua::Nil)?;
        }
        
        // Set global variables from context
        for (key, value) in &context.globals {
            let lua_value = self.json_to_lua_value(&lua, value)?;
            lua.globals().set(key.as_str(), lua_value)?;
        }
        
        Ok(lua)
    }
    
    /// Convert JSON value to Lua value
    fn json_to_lua_value<'lua>(&self, lua: &'lua mlua::Lua, value: &serde_json::Value) -> Result<mlua::Value<'lua>> {
        match value {
            serde_json::Value::Null => Ok(mlua::Value::Nil),
            serde_json::Value::Bool(b) => Ok(mlua::Value::Boolean(*b)),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(mlua::Value::Integer(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(mlua::Value::Number(f))
                } else {
                    Ok(mlua::Value::Nil)
                }
            }
            serde_json::Value::String(s) => Ok(mlua::Value::String(lua.create_string(s)?)),
            serde_json::Value::Array(arr) => {
                let table = lua.create_table()?;
                for (i, item) in arr.iter().enumerate() {
                    let lua_value = self.json_to_lua_value(lua, item)?;
                    table.set(i + 1, lua_value)?;
                }
                Ok(mlua::Value::Table(table))
            }
            serde_json::Value::Object(obj) => {
                let table = lua.create_table()?;
                for (key, val) in obj {
                    let lua_value = self.json_to_lua_value(lua, val)?;
                    table.set(key.as_str(), lua_value)?;
                }
                Ok(mlua::Value::Table(table))
            }
        }
    }
    
    /// Convert Lua value to JSON value
    fn lua_value_to_json(&self, value: mlua::Value) -> Result<serde_json::Value> {
        match value {
            mlua::Value::Nil => Ok(serde_json::Value::Null),
            mlua::Value::Boolean(b) => Ok(serde_json::Value::Bool(b)),
            mlua::Value::Integer(i) => Ok(serde_json::Value::Number(serde_json::Number::from(i))),
            mlua::Value::Number(n) => {
                if let Some(num) = serde_json::Number::from_f64(n) {
                    Ok(serde_json::Value::Number(num))
                } else {
                    Ok(serde_json::Value::Null)
                }
            }
            mlua::Value::String(s) => Ok(serde_json::Value::String(s.to_str()?.to_string())),
            mlua::Value::Table(table) => {
                // Simple conversion - treat as object
                let mut object = serde_json::Map::new();
                for pair in table.pairs::<String, mlua::Value>() {
                    let (key, value) = pair?;
                    object.insert(key, self.lua_value_to_json(value)?);
                }
                Ok(serde_json::Value::Object(object))
            }
            _ => Ok(serde_json::Value::Null),
        }
    }
}

#[async_trait::async_trait]
impl ScriptingEngine for LuaEngine {
    fn name(&self) -> &str {
        "Lua Engine"
    }
    
    fn language(&self) -> &str {
        "lua"
    }
    
    async fn execute(&self, code: &str, context: ScriptContext) -> Result<ScriptResult> {
        // For now, use synchronous execution to avoid Send trait issues
        let start_time = Instant::now();
        let lua = self.create_lua(&context)?;
        
        let mut errors = Vec::new();
        
        // Set up print function to capture output
        let output_capture = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let output_capture_clone = output_capture.clone();
        
        let print_fn = lua.create_function(move |_, args: mlua::MultiValue| {
            let mut output_guard = output_capture_clone.lock().unwrap();
            let output_str = args.iter()
                .map(|v| format!("{:?}", v))
                .collect::<Vec<_>>()
                .join("\t");
            output_guard.push(output_str);
            Ok(())
        })?;
        
        lua.globals().set("print", print_fn)?;
        
        // Execute the code synchronously for now
        let result = lua.load(code).exec();
        
        // Get captured output
        let output = output_capture.lock().unwrap().clone();
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        let (success, return_value) = match result {
            Ok(_) => {
                // Try to get return value if it's an expression
                let return_result = lua.load(code).eval::<mlua::Value>();
                match return_result {
                    Ok(value) => (true, Some(self.lua_value_to_json(value).unwrap_or(serde_json::Value::Null))),
                    Err(_) => (true, None),
                }
            }
            Err(e) => {
                errors.push(format!("Lua error: {}", e));
                (false, None)
            }
        };
        
        Ok(ScriptResult {
            success,
            return_value,
            output,
            errors,
            execution_time_ms: execution_time,
            memory_usage_bytes: None,
            statistics: ScriptStatistics {
                lines_executed: code.lines().count() as u32,
                function_calls: 0,
                variables_created: 0,
                peak_memory_bytes: None,
            },
        })
    }
    
    async fn execute_file(&self, file_path: &str, context: ScriptContext) -> Result<ScriptResult> {
        let code = tokio::fs::read_to_string(file_path).await?;
        self.execute(&code, context).await
    }
    
    async fn validate_syntax(&self, code: &str) -> Result<SyntaxValidation> {
        let lua = mlua::Lua::new();
        
        // Try to load the code without executing it
        let result = lua.load(code).into_function();
        match result {
            Ok(_) => Ok(SyntaxValidation {
                is_valid: true,
                errors: vec![],
                warnings: vec![],
            }),
            Err(e) => {
                let error_msg = format!("{}", e);
                let syntax_error = SyntaxError {
                    message: error_msg,
                    line: 1, // TODO: Parse line number from error
                    column: 1, // TODO: Parse column number from error
                    error_type: "SyntaxError".to_string(),
                };
                
                Ok(SyntaxValidation {
                    is_valid: false,
                    errors: vec![syntax_error],
                    warnings: vec![],
                })
            }
        }
    }
    
    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities {
            supports_sandboxing: true,
            supports_debugging: self.config.enable_debug,
            supports_profiling: false,
            supports_bytecode: true,
            max_execution_time_secs: Some(300), // 5 minutes
            max_memory_bytes: self.config.memory_limit,
            supported_modules: vec![
                "string".to_string(),
                "table".to_string(),
                "math".to_string(),
                "coroutine".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_lua_engine_execution() {
        let config = LuaEngineConfig::default();
        let engine = LuaEngine::new(config);
        let context = ScriptContext::default();
        
        let result = engine.execute("return 2 + 2", context).await.unwrap();
        
        assert!(result.success);
        assert_eq!(result.return_value, Some(serde_json::Value::Number(serde_json::Number::from(4))));
    }
    
    #[tokio::test]
    async fn test_lua_engine_print_capture() {
        let config = LuaEngineConfig::default();
        let engine = LuaEngine::new(config);
        let context = ScriptContext::default();
        
        let result = engine.execute("print('Hello, Lua!')", context).await.unwrap();
        
        assert!(result.success);
        assert!(!result.output.is_empty());
        assert!(result.output[0].contains("Hello, Lua!"));
    }
    
    #[tokio::test]
    async fn test_lua_engine_globals() {
        let config = LuaEngineConfig::default();
        let engine = LuaEngine::new(config);
        
        let mut context = ScriptContext::default();
        context.globals.insert("test_var".to_string(), serde_json::Value::Number(serde_json::Number::from(42)));
        
        let result = engine.execute("return test_var * 2", context).await.unwrap();
        
        assert!(result.success);
        assert_eq!(result.return_value, Some(serde_json::Value::Number(serde_json::Number::from(84))));
    }
    
    #[tokio::test]
    async fn test_lua_syntax_validation() {
        let config = LuaEngineConfig::default();
        let engine = LuaEngine::new(config);
        
        // Valid syntax
        let valid_result = engine.validate_syntax("local x = 5; return x").await.unwrap();
        assert!(valid_result.is_valid);
        
        // Invalid syntax
        let invalid_result = engine.validate_syntax("local x = ; return x").await.unwrap();
        assert!(!invalid_result.is_valid);
        assert!(!invalid_result.errors.is_empty());
    }
}