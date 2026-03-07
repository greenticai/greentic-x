mod profile;

use clap::{Args, Parser, Subcommand, ValueEnum};
use greentic_x_contracts::ContractManifest;
use greentic_x_flow::{
    EvidenceItem, FlowDefinition, FlowEngine, FlowError, NoopViewRenderer, OperationCallStep,
    OperationResult, RenderSource, RenderSpec, ReturnStep, StaticFlowRuntime, Step, ValueSource,
};
use greentic_x_ops::OperationManifest;
use greentic_x_types::{
    ActorRef, InvocationStatus, OperationId, Provenance, ResolverCandidate, ResolverId,
    ResolverResultEnvelope, ResolverStatus, ResourceRef,
};
use jsonschema::validator_for;
use profile::{compile_profile, read_profile, validate_profile};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "gx",
    about = "Greentic-X scaffold, validate, simulate, and inspect tooling"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Contract {
        #[command(subcommand)]
        command: ContractCommand,
    },
    Op {
        #[command(subcommand)]
        command: OpCommand,
    },
    Flow {
        #[command(subcommand)]
        command: FlowCommand,
    },
    Resolver {
        #[command(subcommand)]
        command: ResolverCommand,
    },
    View {
        #[command(subcommand)]
        command: ViewCommand,
    },
    Profile {
        #[command(subcommand)]
        command: ProfileCommand,
    },
    Simulate(SimulateArgs),
    Doctor(DoctorArgs),
    Catalog {
        #[command(subcommand)]
        command: CatalogCommand,
    },
}

#[derive(Subcommand)]
enum ContractCommand {
    New(NewContractArgs),
    Validate(PathArgs),
}

#[derive(Subcommand)]
enum OpCommand {
    New(NewOpArgs),
    Validate(PathArgs),
}

#[derive(Subcommand)]
enum FlowCommand {
    New(NewFlowArgs),
    Validate(PathArgs),
}

#[derive(Subcommand)]
enum ResolverCommand {
    New(NewResolverArgs),
    Validate(PathArgs),
}

#[derive(Subcommand)]
enum ViewCommand {
    New(NewViewArgs),
    Validate(PathArgs),
}

#[derive(Subcommand)]
enum ProfileCommand {
    Validate(PathArgs),
    Compile(CompileProfileArgs),
}

#[derive(Subcommand)]
enum CatalogCommand {
    List(CatalogListArgs),
}

#[derive(Args)]
struct PathArgs {
    path: PathBuf,
}

#[derive(Args)]
struct NewContractArgs {
    path: PathBuf,
    #[arg(long)]
    contract_id: String,
    #[arg(long, default_value = "resource")]
    resource_type: String,
    #[arg(long, default_value = "v1")]
    version: String,
}

#[derive(Args)]
struct NewOpArgs {
    path: PathBuf,
    #[arg(long)]
    operation_id: String,
    #[arg(long, default_value = "gx.resource")]
    contract_id: String,
    #[arg(long, default_value = "v1")]
    version: String,
}

#[derive(Args)]
struct NewFlowArgs {
    path: PathBuf,
    #[arg(long)]
    flow_id: String,
    #[arg(long, default_value = "v1")]
    version: String,
}

#[derive(Args)]
struct NewResolverArgs {
    path: PathBuf,
    #[arg(long)]
    resolver_id: String,
    #[arg(long, default_value = "gx.resolver.result.v1")]
    output_spec: String,
    #[arg(long, default_value = "v1")]
    version: String,
}

#[derive(Args)]
struct NewViewArgs {
    path: PathBuf,
    #[arg(long)]
    view_id: String,
    #[arg(long, default_value = "summary")]
    view_type: String,
    #[arg(long, default_value = "gx.view.v1")]
    spec_ref: String,
    #[arg(long, default_value = "v1")]
    version: String,
}

#[derive(Args)]
struct CompileProfileArgs {
    path: PathBuf,
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(Args)]
struct SimulateArgs {
    path: PathBuf,
    #[arg(long)]
    stubs: Option<PathBuf>,
    #[arg(long)]
    input: Option<PathBuf>,
}

#[derive(Args)]
struct DoctorArgs {
    #[arg(default_value = ".")]
    path: PathBuf,
}

#[derive(Args)]
struct CatalogListArgs {
    #[arg(long)]
    kind: Option<CatalogKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum CatalogKind {
    Contracts,
    Resolvers,
    Ops,
    Views,
    FlowTemplates,
}

#[derive(Debug, Deserialize)]
struct FlowPackageManifest {
    flow_id: String,
    version: String,
    description: String,
    flow: String,
    #[serde(default)]
    stubs: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResolverPackageManifest {
    resolver_id: String,
    version: String,
    description: String,
    query_schema: SchemaFileRef,
    output_spec: String,
}

#[derive(Debug, Deserialize)]
struct ViewPackageManifest {
    view_id: String,
    version: String,
    view_type: String,
    spec_ref: String,
    description: String,
    template: String,
}

#[derive(Debug, Deserialize)]
struct SchemaFileRef {
    schema_id: String,
    version: String,
    #[serde(default)]
    uri: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct SimulationStubs {
    #[serde(default)]
    operations: Vec<OperationStub>,
    #[serde(default)]
    resolvers: Vec<ResolverStub>,
}

#[derive(Debug, Deserialize)]
struct OperationStub {
    operation_id: String,
    #[serde(default)]
    invocation_id: Option<String>,
    output: Value,
    #[serde(default)]
    evidence: Vec<EvidenceItem>,
    #[serde(default)]
    warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ResolverStub {
    resolver_id: String,
    status: ResolverStatus,
    #[serde(default)]
    selected: Option<ResolverStubCandidate>,
    #[serde(default)]
    candidates: Vec<ResolverStubCandidate>,
    #[serde(default)]
    warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ResolverStubCandidate {
    resource: ResourceRef,
    #[serde(default)]
    display: Option<String>,
    #[serde(default)]
    confidence: Option<f64>,
    #[serde(default)]
    metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct CatalogIndex {
    #[serde(default)]
    entries: Vec<Value>,
}

#[derive(Default)]
struct Diagnostics {
    warnings: Vec<String>,
    errors: Vec<String>,
}

impl Diagnostics {
    fn warning(&mut self, message: impl Into<String>) {
        self.warnings.push(message.into());
    }

    fn error(&mut self, message: impl Into<String>) {
        self.errors.push(message.into());
    }

    fn extend(&mut self, other: Diagnostics) {
        self.warnings.extend(other.warnings);
        self.errors.extend(other.errors);
    }

    fn into_result(self, ok_message: impl Into<String>) -> Result<String, String> {
        if self.errors.is_empty() {
            let mut lines = vec![ok_message.into()];
            if !self.warnings.is_empty() {
                lines.push(format!("warnings: {}", self.warnings.len()));
                for warning in self.warnings {
                    lines.push(format!("- {warning}"));
                }
            }
            Ok(lines.join("\n"))
        } else {
            let mut lines = vec![format!("errors: {}", self.errors.len())];
            for error in self.errors {
                lines.push(format!("- {error}"));
            }
            if !self.warnings.is_empty() {
                lines.push(format!("warnings: {}", self.warnings.len()));
                for warning in self.warnings {
                    lines.push(format!("- {warning}"));
                }
            }
            Err(lines.join("\n"))
        }
    }
}

pub fn run<I>(args: I, cwd: std::io::Result<PathBuf>) -> Result<String, String>
where
    I: IntoIterator<Item = OsString>,
{
    let cwd = cwd.map_err(|err| format!("failed to determine current directory: {err}"))?;
    let cli = Cli::try_parse_from(args).map_err(|err| err.to_string())?;
    run_command(cli.command, &cwd)
}

fn run_command(command: Command, cwd: &Path) -> Result<String, String> {
    match command {
        Command::Contract {
            command: ContractCommand::New(args),
        } => {
            let path = cwd.join(&args.path);
            scaffold_contract(path, args)
        }
        Command::Contract {
            command: ContractCommand::Validate(args),
        } => validate_contract_dir(&cwd.join(args.path)).into_result("contract validation passed"),
        Command::Op {
            command: OpCommand::New(args),
        } => {
            let path = cwd.join(&args.path);
            scaffold_op(path, args)
        }
        Command::Op {
            command: OpCommand::Validate(args),
        } => validate_op_dir(&cwd.join(args.path)).into_result("op validation passed"),
        Command::Flow {
            command: FlowCommand::New(args),
        } => {
            let path = cwd.join(&args.path);
            scaffold_flow(path, args)
        }
        Command::Flow {
            command: FlowCommand::Validate(args),
        } => validate_flow_package(&cwd.join(args.path)).into_result("flow validation passed"),
        Command::Resolver {
            command: ResolverCommand::New(args),
        } => {
            let path = cwd.join(&args.path);
            scaffold_resolver(path, args)
        }
        Command::Resolver {
            command: ResolverCommand::Validate(args),
        } => validate_resolver_dir(&cwd.join(args.path)).into_result("resolver validation passed"),
        Command::View {
            command: ViewCommand::New(args),
        } => {
            let path = cwd.join(&args.path);
            scaffold_view(path, args)
        }
        Command::View {
            command: ViewCommand::Validate(args),
        } => validate_view_dir(&cwd.join(args.path)).into_result("view validation passed"),
        Command::Profile {
            command: ProfileCommand::Validate(args),
        } => validate_profile_file(&cwd.join(args.path)).into_result("profile validation passed"),
        Command::Profile {
            command: ProfileCommand::Compile(args),
        } => compile_profile_path(&cwd.join(args.path), args.out.map(|path| cwd.join(path))),
        Command::Simulate(args) => simulate_flow(
            &cwd.join(args.path),
            args.stubs.map(|path| cwd.join(path)),
            args.input.map(|path| cwd.join(path)),
        ),
        Command::Doctor(args) => doctor(&cwd.join(args.path)),
        Command::Catalog {
            command: CatalogCommand::List(args),
        } => list_catalog(cwd, args.kind),
    }
}

fn scaffold_contract(path: PathBuf, args: NewContractArgs) -> Result<String, String> {
    ensure_scaffold_dir(&path)?;
    write_json(
        &path.join("contract.json"),
        &json!({
            "contract_id": args.contract_id,
            "version": args.version,
            "description": "Describe the generic purpose of this contract.",
            "resources": [{
                "resource_type": args.resource_type,
                "schema": {
                    "schema_id": format!("greentic-x://contracts/{}/resources/{}", path_file_name(&path), "resource"),
                    "version": "v1",
                    "uri": "schemas/resource.schema.json"
                },
                "patch_rules": [{"path": "/title", "kind": "allow"}],
                "append_collections": [],
                "transitions": [{"from_state": "new", "to_state": "ready"}]
            }],
            "compatibility": [{
                "schema": {
                    "schema_id": format!("greentic-x://contracts/{}/compatibility", path_file_name(&path)),
                    "version": "v1"
                },
                "mode": "backward_compatible"
            }],
            "event_declarations": [{"event_type": "resource_created"}]
        }),
    )?;
    write_json(
        &path.join("schemas/resource.schema.json"),
        &json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "title": "Generic resource",
            "type": "object",
            "required": ["title", "state"],
            "properties": {
                "title": {"type": "string"},
                "state": {"type": "string"}
            }
        }),
    )?;
    write_json(
        &path.join("examples/resource.json"),
        &json!({
            "title": "Example resource",
            "state": "new"
        }),
    )?;
    fs::write(
        path.join("README.md"),
        "# Contract Package\n\nFill in the contract description, schemas, and examples before publishing.\n",
    )
    .map_err(|err| format!("failed to write README: {err}"))?;
    Ok(format!("scaffolded contract at {}", path.display()))
}

fn scaffold_op(path: PathBuf, args: NewOpArgs) -> Result<String, String> {
    ensure_scaffold_dir(&path)?;
    write_json(
        &path.join("op.json"),
        &json!({
            "operation_id": args.operation_id,
            "version": args.version,
            "description": "Describe the generic purpose of this operation.",
            "input_schema": {
                "schema_id": format!("greentic-x://ops/{}/input", path_file_name(&path)),
                "version": "v1",
                "uri": "schemas/input.schema.json"
            },
            "output_schema": {
                "schema_id": format!("greentic-x://ops/{}/output", path_file_name(&path)),
                "version": "v1",
                "uri": "schemas/output.schema.json"
            },
            "supported_contracts": [{
                "contract_id": args.contract_id,
                "version": "v1"
            }],
            "permissions": [{
                "capability": "resource:read",
                "scope": "generic"
            }],
            "examples": [{
                "name": "basic invocation",
                "input": {"title": "Example resource"},
                "output": {"summary": "Example result"}
            }]
        }),
    )?;
    write_json(
        &path.join("schemas/input.schema.json"),
        &json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "title": {"type": "string"}
            }
        }),
    )?;
    write_json(
        &path.join("schemas/output.schema.json"),
        &json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "summary": {"type": "string"}
            }
        }),
    )?;
    write_json(
        &path.join("examples/example.json"),
        &json!({
            "input": {"title": "Example resource"},
            "output": {"summary": "Example result"}
        }),
    )?;
    fs::write(
        path.join("source.md"),
        "# Source Notes\n\nDocument where the operation logic will come from and any downstream adapters it needs.\n",
    )
    .map_err(|err| format!("failed to write source notes: {err}"))?;
    fs::write(
        path.join("README.md"),
        "# Operation Package\n\nFill in schemas, examples, and downstream adapter details before packaging.\n",
    )
    .map_err(|err| format!("failed to write README: {err}"))?;
    Ok(format!("scaffolded op at {}", path.display()))
}

fn scaffold_flow(path: PathBuf, args: NewFlowArgs) -> Result<String, String> {
    ensure_scaffold_dir(&path)?;
    let operation_id = OperationId::new("present.summary")
        .map_err(|err| format!("failed to build scaffold operation id: {err}"))?;
    let flow = FlowDefinition {
        flow_id: args.flow_id.clone(),
        steps: vec![
            Step::call(
                "present",
                OperationCallStep::new(
                    operation_id,
                    json!({ "summary": "Example summary" }),
                    "present_result",
                ),
            ),
            Step::return_output(
                "return",
                ReturnStep::new(ValueSource::context("present_result.output")).with_render(
                    RenderSpec {
                        renderer_id: "noop.summary".to_owned(),
                        source: RenderSource::EvidenceRefs,
                        view_id: "summary-card".to_owned(),
                        title: "Simulation Summary".to_owned(),
                        summary: "Rendered from the final flow output".to_owned(),
                    },
                ),
            ),
        ],
    };
    write_json(
        &path.join("manifest.json"),
        &json!({
            "flow_id": args.flow_id,
            "version": args.version,
            "description": "Generic GX flow scaffold with stubbed simulation data.",
            "flow": "flow.json",
            "stubs": "stubs.json"
        }),
    )?;
    let flow_value = serde_json::to_value(flow)
        .map_err(|err| format!("failed to serialize flow scaffold: {err}"))?;
    write_json(&path.join("flow.json"), &flow_value)?;
    write_json(
        &path.join("stubs.json"),
        &json!({
            "operations": [{
                "operation_id": "present.summary",
                "output": { "summary": "Example summary" },
                "evidence": [{
                    "evidence_id": "evidence-1",
                    "evidence_type": "summary",
                    "producer": "present.summary",
                    "timestamp": "2026-01-01T00:00:00Z",
                    "summary": "Example evidence emitted during simulation"
                }]
            }]
        }),
    )?;
    fs::write(
        path.join("README.md"),
        "# Flow Package\n\nUse `gx flow validate` and `gx simulate` while iterating on this flow.\n",
    )
    .map_err(|err| format!("failed to write README: {err}"))?;
    Ok(format!("scaffolded flow at {}", path.display()))
}

fn scaffold_resolver(path: PathBuf, args: NewResolverArgs) -> Result<String, String> {
    ensure_scaffold_dir(&path)?;
    write_json(
        &path.join("resolver.json"),
        &json!({
            "resolver_id": args.resolver_id,
            "version": args.version,
            "description": "Describe what this resolver matches and how downstream adapters should implement it.",
            "query_schema": {
                "schema_id": format!("greentic-x://resolvers/{}/query", path_file_name(&path)),
                "version": "v1",
                "uri": "schemas/query.schema.json"
            },
            "output_spec": args.output_spec
        }),
    )?;
    write_json(
        &path.join("schemas/query.schema.json"),
        &json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "query": {"type": "string"}
            }
        }),
    )?;
    write_json(
        &path.join("examples/query.json"),
        &json!({
            "query": "example"
        }),
    )?;
    fs::write(
        path.join("README.md"),
        "# Resolver Package\n\nDocument the matching strategy, evidence sources, and downstream adapter requirements.\n",
    )
    .map_err(|err| format!("failed to write README: {err}"))?;
    Ok(format!("scaffolded resolver at {}", path.display()))
}

fn scaffold_view(path: PathBuf, args: NewViewArgs) -> Result<String, String> {
    ensure_scaffold_dir(&path)?;
    write_json(
        &path.join("view.json"),
        &json!({
            "view_id": args.view_id,
            "version": args.version,
            "view_type": args.view_type,
            "spec_ref": args.spec_ref,
            "description": "Describe the neutral view and downstream channel mappings.",
            "template": "template.json"
        }),
    )?;
    write_json(
        &path.join("template.json"),
        &json!({
            "title": "Replace with a neutral title template",
            "summary": "Replace with a neutral summary template",
            "body": {
                "kind": "table",
                "columns": ["name", "value"]
            }
        }),
    )?;
    fs::write(
        path.join("README.md"),
        "# View Package\n\nDocument how this neutral view maps into downstream channels without coupling GX to one UI surface.\n",
    )
    .map_err(|err| format!("failed to write README: {err}"))?;
    Ok(format!("scaffolded view at {}", path.display()))
}

fn validate_contract_dir(path: &Path) -> Diagnostics {
    let mut diagnostics = Diagnostics::default();
    let manifest_path = path.join("contract.json");
    let manifest = match read_json::<ContractManifest>(&manifest_path) {
        Ok(manifest) => manifest,
        Err(err) => {
            diagnostics.error(err);
            return diagnostics;
        }
    };
    if manifest.version.as_str().is_empty() {
        diagnostics.error(format!(
            "{}: version must not be empty",
            manifest_path.display()
        ));
    }
    for issue in manifest.validate() {
        diagnostics.error(format!("{}: {:?}", manifest_path.display(), issue));
    }
    for resource in &manifest.resources {
        check_schema_uri(
            path,
            resource.schema.uri.as_deref(),
            "resource schema",
            &mut diagnostics,
        );
        for collection in &resource.append_collections {
            check_schema_uri(
                path,
                collection.item_schema.uri.as_deref(),
                "append collection schema",
                &mut diagnostics,
            );
        }
    }
    check_examples_dir(path, &mut diagnostics);
    diagnostics
}

fn validate_op_dir(path: &Path) -> Diagnostics {
    let mut diagnostics = Diagnostics::default();
    let manifest_path = path.join("op.json");
    let manifest = match read_json::<OperationManifest>(&manifest_path) {
        Ok(manifest) => manifest,
        Err(err) => {
            diagnostics.error(err);
            return diagnostics;
        }
    };
    if manifest.version.as_str().is_empty() {
        diagnostics.error(format!(
            "{}: version must not be empty",
            manifest_path.display()
        ));
    }
    for issue in manifest.validate() {
        diagnostics.error(format!("{}: {:?}", manifest_path.display(), issue));
    }
    check_schema_uri(
        path,
        manifest.input_schema.uri.as_deref(),
        "input schema",
        &mut diagnostics,
    );
    check_schema_uri(
        path,
        manifest.output_schema.uri.as_deref(),
        "output schema",
        &mut diagnostics,
    );
    check_examples_dir(path, &mut diagnostics);
    diagnostics
}

fn validate_flow_package(path: &Path) -> Diagnostics {
    let mut diagnostics = Diagnostics::default();
    let (package_root, manifest) = match read_flow_manifest(path) {
        Ok(value) => value,
        Err(err) => {
            diagnostics.error(err);
            return diagnostics;
        }
    };
    if manifest.version.trim().is_empty() {
        diagnostics.error(format!(
            "{}: version metadata is missing",
            package_root.join("manifest.json").display()
        ));
    }
    if manifest.description.trim().is_empty() {
        diagnostics.warning(format!(
            "{}: description is empty",
            package_root.join("manifest.json").display()
        ));
    }
    let flow_path = package_root.join(&manifest.flow);
    let flow = match read_json::<FlowDefinition>(&flow_path) {
        Ok(flow) => flow,
        Err(err) => {
            diagnostics.error(err);
            return diagnostics;
        }
    };
    if flow.flow_id != manifest.flow_id {
        diagnostics.error(format!(
            "{}: flow_id {} does not match manifest flow_id {}",
            flow_path.display(),
            flow.flow_id,
            manifest.flow_id
        ));
    }
    diagnostics.extend(validate_flow_definition(&flow, &flow_path));
    if let Some(stubs) = manifest.stubs.as_deref() {
        let stubs_path = package_root.join(stubs);
        if !stubs_path.exists() {
            diagnostics.error(format!(
                "{}: declared stubs file does not exist",
                stubs_path.display()
            ));
        } else if let Err(err) = read_json::<SimulationStubs>(&stubs_path) {
            diagnostics.error(err);
        }
    }
    diagnostics
}

fn validate_resolver_dir(path: &Path) -> Diagnostics {
    let mut diagnostics = Diagnostics::default();
    let manifest_path = path.join("resolver.json");
    let manifest = match read_json::<ResolverPackageManifest>(&manifest_path) {
        Ok(manifest) => manifest,
        Err(err) => {
            diagnostics.error(err);
            return diagnostics;
        }
    };
    if manifest.resolver_id.trim().is_empty() {
        diagnostics.error(format!(
            "{}: resolver_id must not be empty",
            manifest_path.display()
        ));
    }
    if manifest.version.trim().is_empty() {
        diagnostics.error(format!(
            "{}: version must not be empty",
            manifest_path.display()
        ));
    }
    if manifest.description.trim().is_empty() {
        diagnostics.warning(format!("{}: description is empty", manifest_path.display()));
    }
    if manifest.output_spec.trim().is_empty() {
        diagnostics.error(format!(
            "{}: output_spec must not be empty",
            manifest_path.display()
        ));
    }
    if manifest.query_schema.schema_id.trim().is_empty() {
        diagnostics.error(format!(
            "{}: query_schema.schema_id must not be empty",
            manifest_path.display()
        ));
    }
    if manifest.query_schema.version.trim().is_empty() {
        diagnostics.error(format!(
            "{}: query_schema.version must not be empty",
            manifest_path.display()
        ));
    }
    check_schema_uri(
        path,
        manifest.query_schema.uri.as_deref(),
        "query schema",
        &mut diagnostics,
    );
    if let Some(uri) = manifest.query_schema.uri.as_deref() {
        check_json_schema_file(&path.join(uri), "query schema", &mut diagnostics);
    }
    check_examples_dir(path, &mut diagnostics);
    diagnostics
}

fn validate_view_dir(path: &Path) -> Diagnostics {
    let mut diagnostics = Diagnostics::default();
    let manifest_path = path.join("view.json");
    let manifest = match read_json::<ViewPackageManifest>(&manifest_path) {
        Ok(manifest) => manifest,
        Err(err) => {
            diagnostics.error(err);
            return diagnostics;
        }
    };
    if manifest.view_id.trim().is_empty() {
        diagnostics.error(format!(
            "{}: view_id must not be empty",
            manifest_path.display()
        ));
    }
    if manifest.version.trim().is_empty() {
        diagnostics.error(format!(
            "{}: version must not be empty",
            manifest_path.display()
        ));
    }
    if manifest.view_type.trim().is_empty() {
        diagnostics.error(format!(
            "{}: view_type must not be empty",
            manifest_path.display()
        ));
    }
    if manifest.spec_ref.trim().is_empty() {
        diagnostics.error(format!(
            "{}: spec_ref must not be empty",
            manifest_path.display()
        ));
    }
    if manifest.description.trim().is_empty() {
        diagnostics.warning(format!("{}: description is empty", manifest_path.display()));
    }
    let template_path = path.join(&manifest.template);
    if !template_path.exists() {
        diagnostics.error(format!(
            "{}: template file {} does not exist",
            manifest_path.display(),
            template_path.display()
        ));
    } else {
        match read_json::<Value>(&template_path) {
            Ok(template) => {
                if template.get("title").and_then(Value::as_str).is_none() {
                    diagnostics.error(format!(
                        "{}: template must contain a string title",
                        template_path.display()
                    ));
                }
                if template.get("summary").and_then(Value::as_str).is_none() {
                    diagnostics.error(format!(
                        "{}: template must contain a string summary",
                        template_path.display()
                    ));
                }
            }
            Err(err) => diagnostics.error(err),
        }
    }
    diagnostics
}

fn validate_profile_file(path: &Path) -> Diagnostics {
    let mut diagnostics = Diagnostics::default();
    let profile = match read_profile(path) {
        Ok(profile) => profile,
        Err(err) => {
            diagnostics.error(err);
            return diagnostics;
        }
    };
    for issue in validate_profile(&profile) {
        diagnostics.error(format!("{}: {}", path.display(), issue));
    }
    diagnostics
}

fn compile_profile_path(path: &Path, out: Option<PathBuf>) -> Result<String, String> {
    let profile = read_profile(path)?;
    let flow = compile_profile(&profile)?;
    let output = serde_json::to_value(&flow)
        .map_err(|err| format!("failed to serialize compiled flow: {err}"))?;
    match out {
        Some(path) => {
            write_json(&path, &output)?;
            Ok(format!("compiled profile to {}", path.display()))
        }
        None => serde_json::to_string_pretty(&output)
            .map_err(|err| format!("failed to render compiled flow: {err}")),
    }
}

fn validate_flow_definition(flow: &FlowDefinition, flow_path: &Path) -> Diagnostics {
    let mut diagnostics = Diagnostics::default();
    let mut ids = BTreeSet::new();
    let mut split_ids = BTreeSet::new();
    let mut has_return = false;
    for step in &flow.steps {
        if !ids.insert(step.id.clone()) {
            diagnostics.error(format!(
                "{}: duplicate step id {}",
                flow_path.display(),
                step.id
            ));
        }
        match &step.kind {
            greentic_x_flow::StepKind::Branch(branch) => {
                for case in &branch.cases {
                    if !flow
                        .steps
                        .iter()
                        .any(|candidate| candidate.id == case.next_step_id)
                    {
                        diagnostics.error(format!(
                            "{}: branch {} references missing step {}",
                            flow_path.display(),
                            step.id,
                            case.next_step_id
                        ));
                    }
                }
                if let Some(default) = &branch.default_next_step_id
                    && !flow.steps.iter().any(|candidate| candidate.id == *default)
                {
                    diagnostics.error(format!(
                        "{}: branch {} default references missing step {}",
                        flow_path.display(),
                        step.id,
                        default
                    ));
                }
            }
            greentic_x_flow::StepKind::Split(split) => {
                split_ids.insert(step.id.clone());
                let mut branch_ids = BTreeSet::new();
                for branch in &split.branches {
                    if !branch_ids.insert(branch.branch_id.clone()) {
                        diagnostics.error(format!(
                            "{}: split {} has duplicate branch id {}",
                            flow_path.display(),
                            step.id,
                            branch.branch_id
                        ));
                    }
                    let mut nested_ids = BTreeSet::new();
                    for nested in &branch.steps {
                        if !nested_ids.insert(nested.id.clone()) {
                            diagnostics.error(format!(
                                "{}: split {} branch {} has duplicate nested step id {}",
                                flow_path.display(),
                                step.id,
                                branch.branch_id,
                                nested.id
                            ));
                        }
                    }
                }
            }
            greentic_x_flow::StepKind::Join(join) => {
                if !split_ids.contains(&join.split_step_id) {
                    diagnostics.error(format!(
                        "{}: join {} references missing or later split {}",
                        flow_path.display(),
                        step.id,
                        join.split_step_id
                    ));
                }
            }
            greentic_x_flow::StepKind::Return(return_step) => {
                has_return = true;
                if let Some(render) = &return_step.render {
                    if render.renderer_id.trim().is_empty() {
                        diagnostics.error(format!(
                            "{}: return {} has empty renderer_id",
                            flow_path.display(),
                            step.id
                        ));
                    }
                    if render.view_id.trim().is_empty() {
                        diagnostics.error(format!(
                            "{}: return {} has empty view_id",
                            flow_path.display(),
                            step.id
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    if !has_return {
        diagnostics.error(format!(
            "{}: flow must include at least one return step",
            flow_path.display()
        ));
    }
    diagnostics
}

fn simulate_flow(
    path: &Path,
    stubs_override: Option<PathBuf>,
    input_override: Option<PathBuf>,
) -> Result<String, String> {
    let (package_root, manifest) = read_flow_manifest(path)?;
    let flow_path = package_root.join(&manifest.flow);
    let flow = read_json::<FlowDefinition>(&flow_path)?;
    let input = match input_override {
        Some(path) => read_json::<Value>(&path)?,
        None => {
            let default_input = package_root.join("input.json");
            if default_input.exists() {
                read_json::<Value>(&default_input)?
            } else {
                json!({})
            }
        }
    };
    let stubs_path = match stubs_override {
        Some(path) => path,
        None => package_root.join(
            manifest
                .stubs
                .as_deref()
                .ok_or_else(|| format!("{}: no stubs file configured", flow_path.display()))?,
        ),
    };
    let stubs = read_json::<SimulationStubs>(&stubs_path)?;
    let mut operations = HashMap::new();
    for stub in stubs.operations {
        let operation_id = OperationId::new(stub.operation_id.clone())
            .map_err(|err| format!("invalid operation id {}: {err}", stub.operation_id))?;
        operations.insert(
            stub.operation_id.clone(),
            OperationResult {
                envelope: greentic_x_types::OperationResultEnvelope {
                    invocation_id: stub
                        .invocation_id
                        .unwrap_or_else(|| format!("invoke-{}", stub.operation_id)),
                    operation_id,
                    status: InvocationStatus::Succeeded,
                    output: Some(stub.output),
                    evidence_refs: Vec::new(),
                    warnings: stub.warnings,
                    view_hints: Vec::new(),
                },
                evidence: stub.evidence,
            },
        );
    }
    let mut resolvers = HashMap::new();
    for stub in stubs.resolvers {
        let resolver_id = ResolverId::new(stub.resolver_id.clone())
            .map_err(|err| format!("invalid resolver id {}: {err}", stub.resolver_id))?;
        resolvers.insert(
            stub.resolver_id,
            ResolverResultEnvelope {
                resolver_id,
                status: stub.status,
                selected: stub.selected.map(into_candidate),
                candidates: stub.candidates.into_iter().map(into_candidate).collect(),
                warnings: stub.warnings,
            },
        );
    }

    let provenance = Provenance::new(
        ActorRef::service("gx-cli").map_err(|err| format!("invalid actor id gx-cli: {err}"))?,
    );
    let mut runtime = StaticFlowRuntime::with_operations(operations);
    for (resolver_id, result) in resolvers {
        runtime.insert_resolver(resolver_id, result);
    }
    let mut evidence_store = greentic_x_flow::InMemoryEvidenceStore::default();
    let mut engine = FlowEngine::default();
    let run = engine
        .execute(
            &flow,
            input,
            provenance,
            &mut runtime,
            &mut evidence_store,
            &NoopViewRenderer,
        )
        .map_err(format_flow_error)?;
    serde_json::to_string_pretty(&run).map_err(|err| format!("failed to serialize run: {err}"))
}

fn doctor(path: &Path) -> Result<String, String> {
    let mut diagnostics = Diagnostics::default();
    let contract_dirs = discover_dirs(path, "contracts", "contract.json");
    let op_dirs = discover_dirs(path, "ops", "op.json");
    let resolver_dirs = discover_dirs(path, "resolvers", "resolver.json");
    let view_dirs = discover_dirs(path, "views", "view.json");
    let flow_dirs = discover_dirs(path, "flows", "manifest.json");
    let example_flow_dirs = discover_dirs(path, "examples", "manifest.json");
    let profile_files = discover_files(path, "examples", "profile.json");

    let mut known_contracts = BTreeSet::new();
    for dir in &contract_dirs {
        let manifest_path = dir.join("contract.json");
        if let Ok(manifest) = read_json::<ContractManifest>(&manifest_path) {
            known_contracts.insert(manifest.contract_id.to_string());
        }
        diagnostics.extend(validate_contract_dir(dir));
    }

    let known_resolvers = load_catalog_ids(path, CatalogKind::Resolvers, &["resolver_id"])?;
    let mut known_ops = load_catalog_ids(path, CatalogKind::Ops, &["operation_id"])?;
    for dir in &resolver_dirs {
        let manifest_path = dir.join("resolver.json");
        match read_json::<ResolverPackageManifest>(&manifest_path) {
            Ok(manifest) => {
                if !catalog_entry_exists(
                    path,
                    CatalogKind::Resolvers,
                    "resolver_id",
                    &manifest.resolver_id,
                )? {
                    diagnostics.warning(format!(
                        "{}: resolver {} is not present in catalog/core/resolvers/index.json",
                        manifest_path.display(),
                        manifest.resolver_id
                    ));
                }
            }
            Err(err) => diagnostics.error(err),
        }
        diagnostics.extend(validate_resolver_dir(dir));
    }
    for dir in &op_dirs {
        let manifest_path = dir.join("op.json");
        match read_json::<OperationManifest>(&manifest_path) {
            Ok(manifest) => {
                for supported in &manifest.supported_contracts {
                    if !known_contracts.is_empty()
                        && !known_contracts.contains(&supported.contract_id.to_string())
                    {
                        diagnostics.error(format!(
                            "{}: supported contract {} is not present under contracts/",
                            manifest_path.display(),
                            supported.contract_id
                        ));
                    }
                }
                known_ops.insert(manifest.operation_id.to_string());
            }
            Err(err) => diagnostics.error(err),
        }
        diagnostics.extend(validate_op_dir(dir));
    }

    let known_views = load_catalog_ids(path, CatalogKind::Views, &["view_id"])?;
    for dir in &view_dirs {
        let manifest_path = dir.join("view.json");
        match read_json::<ViewPackageManifest>(&manifest_path) {
            Ok(manifest) => {
                if !catalog_entry_exists(path, CatalogKind::Views, "view_id", &manifest.view_id)? {
                    diagnostics.warning(format!(
                        "{}: view {} is not present in catalog/core/views/index.json",
                        manifest_path.display(),
                        manifest.view_id
                    ));
                }
            }
            Err(err) => diagnostics.error(err),
        }
        diagnostics.extend(validate_view_dir(dir));
    }
    for dir in flow_dirs.iter().chain(example_flow_dirs.iter()) {
        diagnostics.extend(validate_flow_package(dir));
        if let Ok((package_root, manifest)) = read_flow_manifest(dir) {
            let flow_path = package_root.join(&manifest.flow);
            if let Ok(flow) = read_json::<FlowDefinition>(&flow_path) {
                for step in &flow.steps {
                    match &step.kind {
                        greentic_x_flow::StepKind::Resolve(resolve) => {
                            if !known_resolvers.contains(&resolve.resolver_id.to_string()) {
                                diagnostics.error(format!(
                                    "{}: step {} references unknown resolver {}",
                                    flow_path.display(),
                                    step.id,
                                    resolve.resolver_id
                                ));
                            }
                        }
                        greentic_x_flow::StepKind::Call(call) => {
                            if !known_ops.contains(&call.operation_id.to_string()) {
                                diagnostics.error(format!(
                                    "{}: step {} references unknown operation {}",
                                    flow_path.display(),
                                    step.id,
                                    call.operation_id
                                ));
                            }
                        }
                        greentic_x_flow::StepKind::Return(return_step) => {
                            if let Some(render) = &return_step.render
                                && !known_views.is_empty()
                                && !known_views.contains(&render.view_id)
                            {
                                diagnostics.warning(format!(
                                    "{}: return step {} uses non-catalog view {}",
                                    flow_path.display(),
                                    step.id,
                                    render.view_id
                                ));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    for profile_path in &profile_files {
        diagnostics.extend(validate_profile_file(profile_path));
        if let Ok(profile) = read_profile(profile_path) {
            match compile_profile(&profile) {
                Ok(compiled) => {
                    let flow_path = profile_path
                        .parent()
                        .map(|parent| parent.join("flow.json"))
                        .unwrap_or_else(|| PathBuf::from("flow.json"));
                    if flow_path.exists() {
                        match read_json::<FlowDefinition>(&flow_path) {
                            Ok(existing) => {
                                if existing != compiled {
                                    diagnostics.error(format!(
                                        "{}: compiled profile output differs from checked-in flow.json",
                                        profile_path.display()
                                    ));
                                }
                            }
                            Err(err) => diagnostics.error(err),
                        }
                    }
                }
                Err(err) => diagnostics.error(format!("{}: {err}", profile_path.display())),
            }
        }
    }

    diagnostics.into_result("doctor checks passed")
}

fn list_catalog(cwd: &Path, kind: Option<CatalogKind>) -> Result<String, String> {
    let kinds = match kind {
        Some(kind) => vec![kind],
        None => vec![
            CatalogKind::Contracts,
            CatalogKind::Resolvers,
            CatalogKind::Ops,
            CatalogKind::Views,
            CatalogKind::FlowTemplates,
        ],
    };
    let mut lines = Vec::new();
    for kind in kinds {
        let index_path = catalog_index_path(cwd, kind);
        let index = read_json::<CatalogIndex>(&index_path)?;
        lines.push(format!("[{}]", catalog_kind_name(kind)));
        for entry in index.entries {
            let summary = entry_summary(&entry);
            lines.push(format!("- {summary}"));
        }
    }
    Ok(lines.join("\n"))
}

fn load_catalog_ids(
    root: &Path,
    kind: CatalogKind,
    preferred_keys: &[&str],
) -> Result<BTreeSet<String>, String> {
    let index = read_json::<CatalogIndex>(&catalog_index_path(root, kind))?;
    let mut ids = BTreeSet::new();
    for entry in index.entries {
        for key in preferred_keys {
            if let Some(value) = entry.get(*key).and_then(Value::as_str) {
                ids.insert(value.to_owned());
                break;
            }
        }
    }
    Ok(ids)
}

fn catalog_entry_exists(
    root: &Path,
    kind: CatalogKind,
    key: &str,
    expected: &str,
) -> Result<bool, String> {
    let index = read_json::<CatalogIndex>(&catalog_index_path(root, kind))?;
    Ok(index.entries.iter().any(|entry| {
        entry
            .get(key)
            .and_then(Value::as_str)
            .map(|value| value == expected)
            .unwrap_or(false)
    }))
}

fn discover_dirs(root: &Path, container: &str, marker: &str) -> Vec<PathBuf> {
    if root.join(marker).exists() {
        return vec![root.to_path_buf()];
    }
    let base = root.join(container);
    let Ok(entries) = fs::read_dir(&base) else {
        return Vec::new();
    };
    let mut dirs = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.join(marker).exists())
        .collect::<Vec<_>>();
    dirs.sort();
    dirs
}

fn discover_files(root: &Path, container: &str, marker: &str) -> Vec<PathBuf> {
    let base = root.join(container);
    let Ok(entries) = fs::read_dir(&base) else {
        return Vec::new();
    };
    let mut files = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .map(|path| path.join(marker))
        .filter(|path| path.exists())
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn read_flow_manifest(path: &Path) -> Result<(PathBuf, FlowPackageManifest), String> {
    let package_root = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .ok_or_else(|| format!("{}: cannot determine parent directory", path.display()))?
            .to_path_buf()
    };
    let manifest_path = package_root.join("manifest.json");
    let manifest = read_json::<FlowPackageManifest>(&manifest_path)?;
    Ok((package_root, manifest))
}

fn read_json<T>(path: &Path) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let data = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&data).map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(value)
        .map_err(|err| format!("failed to serialize {}: {err}", path.display()))?;
    fs::write(path, content).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn ensure_scaffold_dir(path: &Path) -> Result<(), String> {
    if path.exists() {
        let mut entries = fs::read_dir(path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        if entries.next().is_some() {
            return Err(format!(
                "{} already exists and is not empty",
                path.display()
            ));
        }
    } else {
        fs::create_dir_all(path)
            .map_err(|err| format!("failed to create {}: {err}", path.display()))?;
    }
    fs::create_dir_all(path.join("schemas"))
        .map_err(|err| format!("failed to create schemas dir: {err}"))?;
    fs::create_dir_all(path.join("examples"))
        .map_err(|err| format!("failed to create examples dir: {err}"))?;
    Ok(())
}

fn path_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "package".to_owned())
}

fn check_schema_uri(path: &Path, uri: Option<&str>, label: &str, diagnostics: &mut Diagnostics) {
    match uri {
        Some(uri) => {
            let schema_path = path.join(uri);
            if !schema_path.exists() {
                diagnostics.error(format!(
                    "{}: {} file {} does not exist",
                    path.display(),
                    label,
                    schema_path.display()
                ));
            }
        }
        None => diagnostics.warning(format!("{}: {label} uri is not set", path.display())),
    }
}

fn check_examples_dir(path: &Path, diagnostics: &mut Diagnostics) {
    let examples_dir = path.join("examples");
    let Ok(entries) = fs::read_dir(&examples_dir) else {
        diagnostics.error(format!(
            "{}: examples directory is missing",
            examples_dir.display()
        ));
        return;
    };
    let count = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
        .count();
    if count == 0 {
        diagnostics.error(format!(
            "{}: examples directory does not contain any json examples",
            examples_dir.display()
        ));
    }
}

fn check_json_schema_file(path: &Path, label: &str, diagnostics: &mut Diagnostics) {
    match read_json::<Value>(path) {
        Ok(schema) => {
            if let Err(err) = validator_for(&schema) {
                diagnostics.error(format!(
                    "{}: {label} is not a valid JSON Schema: {err}",
                    path.display()
                ));
            }
        }
        Err(err) => diagnostics.error(err),
    }
}

fn catalog_index_path(root: &Path, kind: CatalogKind) -> PathBuf {
    let suffix = match kind {
        CatalogKind::Contracts => "contracts",
        CatalogKind::Resolvers => "resolvers",
        CatalogKind::Ops => "ops",
        CatalogKind::Views => "views",
        CatalogKind::FlowTemplates => "flow-templates",
    };
    root.join("catalog")
        .join("core")
        .join(suffix)
        .join("index.json")
}

fn catalog_kind_name(kind: CatalogKind) -> &'static str {
    match kind {
        CatalogKind::Contracts => "contracts",
        CatalogKind::Resolvers => "resolvers",
        CatalogKind::Ops => "ops",
        CatalogKind::Views => "views",
        CatalogKind::FlowTemplates => "flow-templates",
    }
}

fn entry_summary(entry: &Value) -> String {
    let ordered = [
        "entry_id",
        "resolver_id",
        "operation_id",
        "view_id",
        "template_id",
    ];
    for key in ordered {
        if let Some(value) = entry.get(key).and_then(Value::as_str) {
            return value.to_owned();
        }
    }
    match serde_json::to_string(entry) {
        Ok(value) => value,
        Err(_) => "<invalid-entry>".to_owned(),
    }
}

fn into_candidate(candidate: ResolverStubCandidate) -> ResolverCandidate {
    ResolverCandidate {
        resource: candidate.resource,
        display: candidate.display,
        confidence: candidate.confidence,
        metadata: candidate.metadata,
    }
}

fn format_flow_error(err: FlowError) -> String {
    match err {
        FlowError::InvalidFlow(message)
        | FlowError::MissingValue(message)
        | FlowError::MissingStep(message)
        | FlowError::Resolver(message)
        | FlowError::Operation(message)
        | FlowError::Join(message)
        | FlowError::Render(message)
        | FlowError::Evidence(message) => message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use tempfile::TempDir;

    fn run_ok(args: &[&str], cwd: &Path) -> Result<String, String> {
        let argv = std::iter::once("gx".to_owned())
            .chain(args.iter().map(|item| (*item).to_owned()))
            .map(OsString::from)
            .collect::<Vec<_>>();
        run(argv, Ok(cwd.to_path_buf()))
    }

    #[test]
    fn scaffolds_contract_op_flow_resolver_and_view() -> Result<(), Box<dyn Error>> {
        let temp = TempDir::new()?;
        let cwd = temp.path();

        let result = run_ok(
            &[
                "contract",
                "new",
                "contracts/example-contract",
                "--contract-id",
                "gx.example",
                "--resource-type",
                "example",
            ],
            cwd,
        )?;
        assert!(result.contains("scaffolded contract"));
        let contract = fs::read_to_string(cwd.join("contracts/example-contract/contract.json"))?;
        assert!(contract.contains("\"contract_id\": \"gx.example\""));

        let result = run_ok(
            &[
                "op",
                "new",
                "ops/example-op",
                "--operation-id",
                "analyse.example",
                "--contract-id",
                "gx.example",
            ],
            cwd,
        )?;
        assert!(result.contains("scaffolded op"));
        let op = fs::read_to_string(cwd.join("ops/example-op/op.json"))?;
        assert!(op.contains("\"operation_id\": \"analyse.example\""));

        let result = run_ok(
            &[
                "flow",
                "new",
                "flows/example-flow",
                "--flow-id",
                "example.flow",
            ],
            cwd,
        )?;
        assert!(result.contains("scaffolded flow"));
        let flow = fs::read_to_string(cwd.join("flows/example-flow/flow.json"))?;
        assert!(flow.contains("\"flow_id\": \"example.flow\""));

        let result = run_ok(
            &[
                "resolver",
                "new",
                "resolvers/example-resolver",
                "--resolver-id",
                "resolve.example",
            ],
            cwd,
        )?;
        assert!(result.contains("scaffolded resolver"));
        let resolver = fs::read_to_string(cwd.join("resolvers/example-resolver/resolver.json"))?;
        assert!(resolver.contains("\"resolver_id\": \"resolve.example\""));

        let result = run_ok(
            &[
                "view",
                "new",
                "views/example-view",
                "--view-id",
                "summary-card",
            ],
            cwd,
        )?;
        assert!(result.contains("scaffolded view"));
        let view = fs::read_to_string(cwd.join("views/example-view/view.json"))?;
        assert!(view.contains("\"view_id\": \"summary-card\""));

        let resolver_validation =
            run_ok(&["resolver", "validate", "resolvers/example-resolver"], cwd)?;
        assert!(resolver_validation.contains("resolver validation passed"));

        let view_validation = run_ok(&["view", "validate", "views/example-view"], cwd)?;
        assert!(view_validation.contains("view validation passed"));
        Ok(())
    }

    #[test]
    fn validates_and_simulates_scaffolded_flow() -> Result<(), Box<dyn Error>> {
        let temp = TempDir::new()?;
        let cwd = temp.path();
        let _ = run_ok(
            &[
                "flow",
                "new",
                "flows/example-flow",
                "--flow-id",
                "example.flow",
            ],
            cwd,
        )?;

        let validation = run_ok(&["flow", "validate", "flows/example-flow"], cwd)?;
        assert!(validation.contains("flow validation passed"));

        let output = run_ok(&["simulate", "flows/example-flow"], cwd)?;
        assert!(
            output.contains("\"status\": \"succeeded\"")
                || output.contains("\"status\": \"partial\"")
        );
        assert!(output.contains("\"view_id\": \"summary-card\""));
        Ok(())
    }

    #[test]
    fn compiles_observability_profiles() -> Result<(), Box<dyn Error>> {
        let temp = TempDir::new()?;
        let cwd = temp.path();
        fs::create_dir_all(cwd.join("profiles"))?;
        write_json(
            &cwd.join("profiles/example.json"),
            &json!({
                "profile_id": "example.profile",
                "resolver": "resolve.by_name",
                "query_ops": ["query.resource"],
                "analysis_ops": ["analyse.threshold"],
                "present_op": "present.summary",
                "split_join": null
            }),
        )?;
        let output = run_ok(&["profile", "compile", "profiles/example.json"], cwd)?;
        assert!(output.contains("\"flow_id\": \"example.profile\""));

        write_json(
            &cwd.join("profiles/split.json"),
            &json!({
                "profile_id": "split.profile",
                "resolver": "resolve.by_name",
                "query_ops": [],
                "analysis_ops": [],
                "present_op": "present.summary",
                "split_join": {
                    "branches": [
                        {
                            "branch_id": "left",
                            "query_ops": ["query.resource"],
                            "analysis_ops": ["analyse.threshold"]
                        },
                        {
                            "branch_id": "right",
                            "query_ops": ["query.linked"],
                            "analysis_ops": ["analyse.percentile"]
                        }
                    ]
                }
            }),
        )?;
        let output = run_ok(&["profile", "compile", "profiles/split.json"], cwd)?;
        assert!(output.contains("\"type\": \"split\""));
        assert!(output.contains("\"type\": \"join\""));
        Ok(())
    }

    #[test]
    fn generic_reference_examples_simulate_successfully() -> Result<(), Box<dyn Error>> {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .ok_or("failed to resolve repo root")?;
        let example_dirs = [
            "examples/top-contributors-generic",
            "examples/entity-utilisation-generic",
            "examples/change-correlation-generic",
            "examples/root-cause-split-join-generic",
        ];

        for dir in example_dirs {
            let validation = run_ok(&["flow", "validate", dir], repo_root)?;
            assert!(validation.contains("flow validation passed"));

            let simulation = run_ok(&["simulate", dir], repo_root)?;
            let run_value: Value = serde_json::from_str(&simulation)?;
            let expected_view: Value =
                read_json(&repo_root.join(dir).join("expected.view.json")).map_err(io_error)?;
            let expected_evidence: Value =
                read_json(&repo_root.join(dir).join("expected.evidence.json")).map_err(io_error)?;

            assert_eq!(
                run_value["view"], expected_view,
                "unexpected view for {dir}"
            );

            let actual_evidence_ids = run_value["view"]["primary_data_refs"].clone();
            let expected_evidence_ids = expected_evidence
                .as_array()
                .ok_or("expected evidence should be an array")?
                .iter()
                .map(|item| item["evidence_id"].clone())
                .collect::<Vec<_>>();
            assert_eq!(
                actual_evidence_ids,
                Value::Array(expected_evidence_ids),
                "unexpected evidence refs for {dir}"
            );
        }
        Ok(())
    }

    #[test]
    fn doctor_catches_broken_references() -> Result<(), Box<dyn Error>> {
        let temp = TempDir::new()?;
        let cwd = temp.path();

        fs::create_dir_all(cwd.join("catalog/core/resolvers"))?;
        fs::create_dir_all(cwd.join("catalog/core/ops"))?;
        fs::create_dir_all(cwd.join("catalog/core/views"))?;
        write_json(
            &cwd.join("catalog/core/resolvers/index.json"),
            &json!({"entries": []}),
        )?;
        write_json(
            &cwd.join("catalog/core/ops/index.json"),
            &json!({"entries": []}),
        )?;
        write_json(
            &cwd.join("catalog/core/views/index.json"),
            &json!({"entries": []}),
        )?;

        let _ = run_ok(
            &[
                "flow",
                "new",
                "flows/example-flow",
                "--flow-id",
                "example.flow",
            ],
            cwd,
        )?;
        let doctor = run_ok(&["doctor", "."], cwd);
        assert!(doctor.is_err());
        let message = match doctor {
            Ok(value) => value,
            Err(err) => err,
        };
        assert!(message.contains("unknown operation"));
        Ok(())
    }

    #[test]
    fn flow_validation_catches_broken_join() -> Result<(), Box<dyn Error>> {
        let temp = TempDir::new()?;
        let cwd = temp.path();
        fs::create_dir_all(cwd.join("flows/broken-flow"))?;
        write_json(
            &cwd.join("flows/broken-flow/manifest.json"),
            &json!({
                "flow_id": "broken.flow",
                "version": "v1",
                "description": "broken",
                "flow": "flow.json"
            }),
        )?;
        write_json(
            &cwd.join("flows/broken-flow/flow.json"),
            &json!({
                "flow_id": "broken.flow",
                "steps": [
                    {
                        "id": "join",
                        "kind": {
                            "type": "join",
                            "split_step_id": "missing-split",
                            "mode": "all",
                            "output_key": "merged"
                        }
                    },
                    {
                        "id": "return",
                        "kind": {
                            "type": "return",
                            "output": {"kind": "literal", "value": {"ok": true}}
                        }
                    }
                ]
            }),
        )?;

        let validation = run_ok(&["flow", "validate", "flows/broken-flow"], cwd);
        assert!(validation.is_err());
        let message = match validation {
            Ok(value) => value,
            Err(err) => err,
        };
        assert!(message.contains("references missing or later split"));
        Ok(())
    }

    fn io_error(message: String) -> Box<dyn Error> {
        Box::new(std::io::Error::other(message))
    }
}
