use crate::prelude::*;
use nu_engine::{script, WholeStreamCommand};

use nu_errors::ShellError;
use nu_path::{canonicalize, canonicalize_with};
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;

use std::path::Path;

pub struct Source;

#[derive(Deserialize)]
pub struct SourceArgs {
    pub filename: Tagged<String>,
}

impl WholeStreamCommand for Source {
    fn name(&self) -> &str {
        "source"
    }

    fn signature(&self) -> Signature {
        Signature::build("source").required(
            "filename",
            SyntaxShape::FilePath,
            "the filepath to the script file to source",
        )
    }

    fn usage(&self) -> &str {
        "Runs a script file in the current context."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        source(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}

pub fn source(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctx = &args.context;
    let filename: Tagged<String> = args.req(0)?;

    let source_file = Path::new(&filename.item);

    // Note: this is a special case for setting the context from a command
    // In this case, if we don't set it now, we'll lose the scope that this
    // variable should be set into.

    let lib_dirs = &ctx
        .configs()
        .lock()
        .global_config
        .as_ref()
        .map(|configuration| match configuration.var("lib_dirs") {
            Some(paths) => paths
                .table_entries()
                .cloned()
                .map(|path| path.as_string())
                .collect(),
            None => vec![],
        });

    if let Some(dir) = lib_dirs {
        for lib_path in dir {
            match lib_path {
                Ok(name) => {
                    let path = if let Ok(p) = canonicalize_with(&source_file, name) {
                        p
                    } else {
                        continue;
                    };

                    if let Ok(contents) = std::fs::read_to_string(path) {
                        let result = script::run_script_standalone(contents, true, ctx, false);

                        if let Err(err) = result {
                            ctx.error(err);
                        }
                        return Ok(OutputStream::empty());
                    }
                }
                Err(reason) => {
                    ctx.error(reason.clone());
                }
            }
        }
    }

    let path = canonicalize(source_file).map_err(|e| {
        ShellError::labeled_error(
            format!("Can't load source file. Reason: {}", e),
            "Can't load this file",
            filename.span(),
        )
    })?;

    let contents = std::fs::read_to_string(path);

    match contents {
        Ok(contents) => {
            let result = script::run_script_standalone(contents, true, ctx, false);

            if let Err(err) = result {
                ctx.error(err);
            }
            Ok(OutputStream::empty())
        }
        Err(e) => {
            ctx.error(ShellError::labeled_error(
                format!("Can't load source file. Reason: {}", e),
                "Can't load this file",
                filename.span(),
            ));

            Ok(OutputStream::empty())
        }
    }
}
