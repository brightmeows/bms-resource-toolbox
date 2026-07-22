//! Core types for interactive command parameter definitions.

use std::path::PathBuf;

/// Semantic constraint for path-type parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathSemantic {
    /// Any path that exists as a directory.
    Any,
    /// Path must be a directory that does NOT directly contain chart files (`.bms`, `.bme`, `.bml`, `.pms`, `.bmson`).
    RootDir,
    /// Path must be a directory that directly contains at least one chart file.
    #[expect(dead_code, reason = "Future use for work-dir validation")]
    WorkDir,
    /// Path must NOT exist (used for new output directories).
    NonExistent,
    /// Path must exist as a file.
    #[expect(dead_code, reason = "Future use for file-selection commands")]
    File,
}

/// The type of a parameter, determining how it's prompted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamType {
    /// Directory or file path with optional semantic constraint.
    Path {
        /// Semantic validation constraint.
        semantic: PathSemantic,
    },
    /// Integer value with optional bounds.
    Int {
        /// Minimum allowed value (inclusive), if any.
        min: Option<i32>,
        /// Maximum allowed value (inclusive), if any.
        max: Option<i32>,
    },
    /// Free-form string input.
    #[expect(dead_code, reason = "Future use for commands with string params")]
    String,
}

/// A parameter definition for an interactive command.
#[derive(Debug, Clone)]
pub struct ParamDef {
    /// The type of the parameter and how to prompt it.
    pub param_type: ParamType,
    /// Human-readable description shown to the user.
    pub description: &'static str,
}

#[expect(
    dead_code,
    reason = "convenience constructors, not all used yet"
)]
impl ParamDef {
    /// Create a path parameter with the given semantic and description.
    #[must_use]
    pub fn path(semantic: PathSemantic, description: &'static str) -> Self {
        Self {
            param_type: ParamType::Path { semantic },
            description,
        }
    }

    /// Create a `RootDir` path parameter.
    #[must_use]
    pub fn root_dir(description: &'static str) -> Self {
        Self::path(PathSemantic::RootDir, description)
    }

    /// Create a `WorkDir` path parameter.
    #[must_use]
    pub fn work_dir(description: &'static str) -> Self {
        Self::path(PathSemantic::WorkDir, description)
    }

    /// Create a `NonExistent` path parameter.
    #[must_use]
    pub fn new_dir(description: &'static str) -> Self {
        Self::path(PathSemantic::NonExistent, description)
    }

    /// Create a `File` path parameter.
    #[must_use]
    pub fn file(description: &'static str) -> Self {
        Self::path(PathSemantic::File, description)
    }

    /// Create an integer parameter with optional bounds.
    #[must_use]
    pub fn int(min: Option<i32>, max: Option<i32>, description: &'static str) -> Self {
        Self {
            param_type: ParamType::Int { min, max },
            description,
        }
    }

    /// Create a string parameter.
    #[must_use]
    pub fn string(description: &'static str) -> Self {
        Self {
            param_type: ParamType::String,
            description,
        }
    }
}

/// The value of a resolved parameter, to be passed to domain functions.
#[derive(Debug, Clone)]
pub enum ParamValue {
    /// A resolved path value.
    Path(PathBuf),
    /// A resolved integer value.
    Int(i32),
    /// A resolved string value.
    #[expect(dead_code, reason = "Future use")]
    String(String),
}

impl ParamValue {
    /// Extract as `PathBuf`, panics if not a Path variant.
    #[must_use]
    pub fn into_path(self) -> PathBuf {
        match self {
            Self::Path(p) => p,
            _ => panic!("ParamValue is not a Path"),
        }
    }

    /// Extract as `i32`, panics if not an Int variant.
    #[must_use]
    pub fn into_int(self) -> i32 {
        match self {
            Self::Int(n) => n,
            _ => panic!("ParamValue is not an Int"),
        }
    }

    /// Extract as `String`, panics if not a String variant.
    #[expect(dead_code, reason = "Future use")]
    #[must_use]
    pub fn into_string(self) -> String {
        match self {
            Self::String(s) => s,
            _ => panic!("ParamValue is not a String"),
        }
    }

    /// Try to extract as a reference to `PathBuf`.
    #[expect(dead_code, reason = "Future use")]
    #[must_use]
    pub fn as_path(&self) -> &PathBuf {
        match self {
            Self::Path(p) => p,
            _ => panic!("ParamValue is not a Path"),
        }
    }
}
