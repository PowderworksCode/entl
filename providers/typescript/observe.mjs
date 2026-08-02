// Observes resolved TypeScript semantics by running the compiler's checker.
//
// Syntax alone cannot say what a call runs on. `xs.filter(p)[0]` is a search
// worth replacing when `xs` is an array and is nothing of the kind when `xs` is
// a `Set` or a user-defined type that happens to spell a method `filter`. The
// difference is not in the source; it is in the type, and only a checker knows
// it.
//
// This observes; it does not decide. Whether a resolved type matters is a
// question for a consumer, and the schema it writes says nothing about
// TypeScript.
//
// Usage:
//   node observe.mjs --project <dir> --out <file.json> [--typescript <lib>]

import path from "node:path";
import fs from "node:fs";

const SCHEMA = 1;

function arg(name, fallback) {
  const at = process.argv.indexOf(`--${name}`);
  return at === -1 ? fallback : process.argv[at + 1];
}

const projectRoot = path.resolve(arg("project", process.cwd()));
const outPath = arg("out");
if (!outPath) {
  console.error("observe.mjs: --out is required, and says where observations go");
  process.exit(1);
}

// The checker is the compiler the project itself builds with wherever possible,
// because observations describe that build and not some other one.
const typescriptPath =
  arg("typescript") ??
  path.join(projectRoot, "node_modules", "typescript", "lib", "typescript.js");
let ts;
try {
  ts = (await import(typescriptPath)).default;
} catch (error) {
  console.error(`observe.mjs: loading the TypeScript compiler at ${typescriptPath}: ${error.message}`);
  process.exit(1);
}

/** Every observation is relative to the tree being observed. */
function relative(fileName) {
  const rel = path.relative(projectRoot, fileName);
  return rel.startsWith("..") ? fileName : rel;
}

function spanOf(node) {
  const file = node.getSourceFile();
  const start = file.getLineAndCharacterOfPosition(node.getStart(file));
  const end = file.getLineAndCharacterOfPosition(node.getEnd());
  return {
    path: relative(file.fileName),
    start_line: start.line + 1,
    start_column: start.character + 1,
    end_line: end.line + 1,
    end_column: end.character + 1,
  };
}

// --- identity -------------------------------------------------------------
//
// A name is not identity here. `getFullyQualifiedName` embeds an absolute path
// for anything a module exports, spells the same symbol differently depending
// on whether it is exported, and gives every anonymous object type the same
// `__type`. A declaration site is none of those things, so entities are minted
// from where they are written and the readable name is carried beside it.

function packageOf(fileName) {
  const at = fileName.lastIndexOf("node_modules/");
  if (at === -1) return null;
  const rest = fileName.slice(at + "node_modules/".length).split("/");
  return rest[0].startsWith("@") ? `${rest[0]}/${rest[1]}` : rest[0];
}

function entityId(decl) {
  const file = decl.getSourceFile();
  const name = file.fileName;
  // A declaration in the standard library is the same declaration in every
  // project, so it is named by what it declares rather than by where it sits.
  if (/\/lib\.[^/]*\.d\.ts$/.test(name)) {
    return `lib:${qualified(decl)}`;
  }
  const pkg = packageOf(name);
  if (pkg) return `pkg:${pkg}:${qualified(decl)}`;
  const at = file.getLineAndCharacterOfPosition(decl.getStart(file));
  return `${relative(name)}:${at.line + 1}:${at.character + 1}`;
}

/** The container-and-member reading a person would use: `Array.filter`. */
function qualified(decl) {
  const own = decl.name?.getText?.() ?? "";
  let parent = decl.parent;
  while (parent) {
    if (
      ts.isInterfaceDeclaration(parent) ||
      ts.isClassDeclaration(parent) ||
      ts.isModuleDeclaration(parent)
    ) {
      const container = parent.name?.getText?.();
      return container ? `${container}.${own}` : own;
    }
    parent = parent.parent;
  }
  return own;
}

function entityKind(decl) {
  if (ts.isMethodSignature(decl) || ts.isMethodDeclaration(decl)) return "method";
  if (ts.isFunctionDeclaration(decl) || ts.isArrowFunction(decl) || ts.isFunctionExpression(decl))
    return "function";
  if (ts.isInterfaceDeclaration(decl)) return "interface";
  if (ts.isClassDeclaration(decl) || ts.isTypeAliasDeclaration(decl)) return "type";
  if (ts.isPropertySignature(decl) || ts.isPropertyDeclaration(decl)) return "field";
  return "other";
}

/** A type named as shallowly as is portable. */
function typeRef(checker, type) {
  const display = checker.typeToString(type);
  // the head is the constructor without its arguments: `Array<string>` is an
  // `Array`, which is the question a consumer actually asks
  const named = display.replace(/<.*$/s, "").replace(/\[\]$/, "").trim();
  // a structural type names no constructor, and saying so is better than
  // reporting its whole spelling as though it were one
  const head = /^[A-Za-z_$][\w$.]*$/.test(named) ? named : "(anonymous)";
  const args = (checker.getTypeArguments?.(type) ?? []).map((argument) => typeRef(checker, argument));
  return {
    // an array literal type prints as `T[]`; its constructor is still Array
    head: /\[\]$/.test(display) ? "Array" : head,
    arguments: args,
    display,
  };
}

// --- the run --------------------------------------------------------------

const configPath = ts.findConfigFile(projectRoot, ts.sys.fileExists, "tsconfig.json");
let fileNames;
let options;
if (configPath) {
  const config = ts.readConfigFile(configPath, ts.sys.readFile);
  const parsed = ts.parseJsonConfigFileContent(config.config ?? {}, ts.sys, path.dirname(configPath));
  fileNames = parsed.fileNames;
  options = parsed.options;
} else {
  // no project file: observe the TypeScript sources that are there, and say so
  fileNames = [];
  const walk = (dir) => {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      if (entry.name === "node_modules" || entry.name.startsWith(".")) continue;
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) walk(full);
      else if (/\.(ts|tsx|mts|cts)$/.test(entry.name) && !entry.name.endsWith(".d.ts"))
        fileNames.push(full);
    }
  };
  walk(projectRoot);
  options = { strict: true, target: ts.ScriptTarget.ES2022, allowJs: false, noEmit: true };
}

const program = ts.createProgram(fileNames, options);
const checker = program.getTypeChecker();

const observations = {
  schema: SCHEMA,
  provenance: {
    provider: "typescript.checker",
    provider_version: "0.0.0",
    toolchain: `typescript ${ts.version}`,
    unit: path.basename(projectRoot),
  },
  // What this provider attempted. Types are the point: resolution alone cannot
  // tell an array from a set, and that distinction is why a checker is run.
  coverage: {
    definitions: true,
    references: false,
    call_edges: true,
    types: true,
    implements: false,
  },
  definitions: [],
  references: [],
  call_edges: [],
  types: [],
  implements: [],
  gaps: [],
};

const seenDefinition = new Set();
function recordDefinition(decl) {
  const id = entityId(decl);
  if (seenDefinition.has(id)) return id;
  seenDefinition.add(id);
  const declared = /\.d\.ts$/.test(decl.getSourceFile().fileName);
  observations.definitions.push({
    id,
    kind: entityKind(decl),
    name: qualified(decl),
    container: null,
    visibility: "unknown",
    // a declaration with no body of its own is not tied to a place a consumer
    // can look at, and inventing one would be worse than saying there is none
    span: declared ? null : spanOf(decl),
  });
  return id;
}

/** The definition a call is written inside. */
function enclosing(node) {
  let parent = node.parent;
  while (parent) {
    if (
      ts.isFunctionDeclaration(parent) ||
      ts.isMethodDeclaration(parent) ||
      ts.isArrowFunction(parent) ||
      ts.isFunctionExpression(parent)
    ) {
      return parent;
    }
    parent = parent.parent;
  }
  return null;
}

for (const source of program.getSourceFiles()) {
  if (source.isDeclarationFile) continue;
  if (!fileNames.includes(source.fileName)) continue;

  const visit = (node) => {
    if (ts.isFunctionDeclaration(node) || ts.isMethodDeclaration(node)) {
      recordDefinition(node);
    }

    if (ts.isCallExpression(node) || ts.isNewExpression(node)) {
      // The receiver's type, at the receiver's span. This is the observation a
      // consumer needs to tell `xs.filter(p)` on an array from the same text
      // on something else that spells a method `filter`.
      if (ts.isCallExpression(node) && ts.isPropertyAccessExpression(node.expression)) {
        const receiver = node.expression.expression;
        try {
          observations.types.push({
            span: spanOf(receiver),
            type_ref: typeRef(checker, checker.getTypeAtLocation(receiver)),
          });
        } catch (error) {
          observations.gaps.push({
            span: spanOf(receiver),
            message: `the type of a receiver could not be read: ${error.message}`,
          });
        }
      }

      const signature = checker.getResolvedSignature(node);
      const decl = signature?.getDeclaration();
      const from = enclosing(node);
      const edge = {
        span: spanOf(node),
        from: from ? entityId(from) : `${relative(source.fileName)}:module`,
        to: [],
        dispatch: "unknown",
      };
      if (decl) {
        edge.to.push(recordDefinition(decl));
        // A property may be declared more than once — overloads, or several
        // types satisfying one interface. The checker picks one; the others
        // are still reachable, and a single destination would overstate what
        // was established.
        const property = ts.isPropertyAccessExpression(node.expression)
          ? checker.getSymbolAtLocation(node.expression.name)
          : undefined;
        const all = property?.declarations ?? [];
        for (const other of all) {
          const id = recordDefinition(other);
          if (!edge.to.includes(id)) edge.to.push(id);
        }
        edge.dispatch = edge.to.length > 1 ? "virtual" : "static";
      } else {
        observations.gaps.push({
          span: spanOf(node),
          message: "a call was found but the checker resolved no signature",
        });
      }
      observations.call_edges.push(edge);
    }
    ts.forEachChild(node, visit);
  };
  visit(source);
}

// Sorted, so the same source and compiler produce byte-identical output.
const key = (value) => JSON.stringify(value);
observations.definitions.sort((a, b) => key(a).localeCompare(key(b)));
observations.call_edges.sort((a, b) => key(a).localeCompare(key(b)));
observations.types.sort((a, b) => key(a).localeCompare(key(b)));
observations.gaps.sort((a, b) => key(a).localeCompare(key(b)));

fs.mkdirSync(path.dirname(path.resolve(outPath)), { recursive: true });
fs.writeFileSync(outPath, `${JSON.stringify(observations, null, 2)}\n`);
console.error(
  `observe.mjs: ${observations.definitions.length} definitions, ` +
    `${observations.call_edges.length} calls, ${observations.types.length} types, ` +
    `${observations.gaps.length} gaps -> ${outPath}`,
);
