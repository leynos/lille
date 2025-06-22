# Harnessing Incremental Computation with DDlog and Rust on Linux

Differential Datalog (DDlog) offers a powerful paradigm for developing programs
that require continuous, incremental computation. This guide provides
comprehensive, accessible documentation for setting up and utilizing DDlog with
the Rust programming language on Linux systems, focusing on the
`vmware-archive/differential-datalog` implementation.

## I. Introduction to Differential Datalog (DDlog)

Understanding the fundamental principles of DDlog is key to leveraging its
capabilities effectively. This section outlines its core concepts, benefits in a
Rust context, and important considerations regarding its development status.

### A. What is DDlog? Core Principles

DDlog is a programming language specifically engineered for incremental
computation. It is particularly well-suited for applications that must
continuously update their outputs in response to changes in their inputs.1 A
hallmark of DDlog is its declarative nature; programmers define the desired
mapping between inputs and outputs, rather than coding the intricate logic of
incremental algorithms themselves.1 This abstraction allows developers to
concentrate on *what* results are needed, while DDlog handles the *how* of
efficiently updating those results as data evolves.

Several key properties define DDlog and its operational characteristics:

- **Relational:** A DDlog program operates by transforming sets of input
  relations (akin to tables in a database) into sets of output relations. This
  makes it a strong fit for applications dealing with relational data, such as
  real-time analytics, cloud management systems, and static program analysis
  tools.1
- **Dataflow-oriented:** At runtime, DDlog programs process a stream of
  updates—insertions, deletions, or modifications—to their input relations. In
  response to each input update, DDlog calculates and outputs the corresponding
  updates to its output relations.1
- **Incremental:** DDlog is designed to perform the minimum amount of
  computation necessary to determine the changes to output relations when inputs
  are modified. This incremental approach can yield significant performance
  advantages for many types of queries.2
- **Bottom-up:** DDlog begins with a set of input facts and systematically
  computes all possible derived facts by applying user-defined rules. This
  "bottom-up" strategy contrasts with "top-down" engines, which are typically
  optimized to answer individual user queries without pre-calculating all
  possible facts.1
- **In-memory:** Data storage and processing in DDlog occur in memory. A common
  use case involves DDlog working alongside a persistent database, where records
  from the database are fed to DDlog as ground facts, and the derived facts
  computed by DDlog are written back.1 Currently, DDlog is designed for datasets
  that can fit entirely within the memory of a single machine.
- **Typed:** To facilitate the creation of safe, clear, and concise code, DDlog
  extends pure Datalog with a robust type system. This includes Booleans,
  unlimited precision integers, bitvectors, floating-point numbers, strings,
  tuples, tagged unions (enums), vectors, sets, and maps. These types can be
  stored in DDlog relations and manipulated by rules, allowing for relational
  operations directly on structured data without prior flattening.1 It also
  supports standard arithmetic operations, a simple procedural language for
  native computations, and string manipulation capabilities.
- **Integrated:** While DDlog programs can be executed interactively through a
  command-line interface, their primary intended use is integration as a library
  within other applications that require deductive database functionality. A
  DDlog program is compiled into a Rust library, which can then be linked
  against programs written in Rust, C/C++, Java, or Go.1

The interplay of DDlog's "declarative," "incremental," and "bottom-up"
characteristics shapes its suitability for specific application domains. Because
programmers define data relationships declaratively 1, and the system processes
changes incrementally 2 to compute all derivable facts in a bottom-up fashion 1,
DDlog excels in scenarios where a complete and continuously updated view of
derived data is essential. This makes it a strong candidate for systems like
real-time monitoring dashboards, complex event processing engines, or any
application where the "world view" must reflect the latest input changes
efficiently and comprehensively.

The "in-memory" nature of DDlog 1 contributes to its performance but also
imposes a constraint on data size. The official documentation notes that, at
present, DDlog is limited to databases that fit within a single machine's memory
and mentions ongoing work towards a distributed version.1 This indicates that
while highly performant for suitable datasets, users must carefully consider if
their data volume aligns with this current limitation.

### B. Why Use DDlog with Rust? Benefits and Use Cases

The integration of DDlog with Rust is a central aspect of its design and
utility. DDlog programs are compiled into Rust libraries 1, a choice that offers
several advantages. Rust's emphasis on performance, memory safety, and its rich
ecosystem make it an excellent host language. By compiling to a Rust library,
DDlog allows developers to incorporate its declarative, incremental data
processing capabilities into high-performance Rust applications.

DDlog's suitability for applications that operate on relational data, such as
real-time analytics, cloud management, and static program analysis tools 1,
aligns well with domains where Rust is increasingly adopted. This synergy allows
Rust developers to delegate complex, evolving data relationship management to
DDlog, while focusing on other aspects of their application in Rust.

The compilation target being a Rust library 1 means that the DDlog logic
effectively becomes native Rust code. As the tutorial documentation clarifies,
"The DDlog compiler generates Rust code, which is compiled by the Rust
compiler...".3 This tight integration ensures that developers can leverage
DDlog's declarative power for intricate data-centric components without a
significant departure from Rust's performance characteristics or its development
ecosystem.

The mention of "static program analysis tools" as a use case 1 is particularly
noteworthy. Given Rust's own sophisticated compiler and features like the borrow
checker, which rely heavily on static analysis, DDlog's rule-based engine
presents a potent tool. It could be employed to model and analyze complex rule
systems, potentially even for developing custom static analyzers or code
intelligence tools within the Rust ecosystem. An internal Rust language team
discussion even explored the idea of generating Datalog output from `rustc` for
analysis purposes, highlighting the conceptual fit.4

### C. A Note on the `vmware-archive/differential-datalog` Repository

It is important for users to be aware of the status of the primary DDlog
software repository. The project is hosted at
`github.com/vmware-archive/differential-datalog`.1 The "archive" prefix in the
repository name typically signifies that the project is no longer under active
development or maintenance by the original team.

The latest official release listed on the repository's releases page is v1.2.3,
dated December 13, 2021.6 This date further suggests a transition away from
active development by the original maintainers.

This archived status implies that users should primarily rely on the existing
documentation and codebase as they are. While DDlog, as it stands, is a
functional and feature-rich tool, prospective users should not anticipate
ongoing feature development, new bug fixes from the original team, or active
support through the original GitHub repository's issue tracker or pull requests.
This consideration may influence the decision to adopt DDlog for new, long-term
critical projects versus its use for research, experimentation, or in contexts
where the existing feature set is sufficient and stable.

## II. Setting Up Your DDlog Environment on Linux

A correct setup is crucial for a smooth experience with DDlog. This section
details the prerequisites, installation methods, and verification steps for
getting DDlog operational on a Linux system.

### A. Prerequisites

Before installing DDlog, ensure your Linux environment meets the following
requirements:

- **Rust Toolchain:** A working Rust installation is essential, as DDlog
  compiles to Rust and is typically integrated into Rust projects.2 The standard
  installation method using `rustup` (available at `https://rustup.rs/`) is
  recommended.

- **Java Development Kit (JDK):** DDlog requires a JDK. A common way to install
  this on Debian-based systems like Ubuntu is `sudo apt install default-jdk`.1
  This dependency might be for parts of DDlog's build system or to support its
  Java language bindings.1

- **Google FlatBuffers Library (Version 1.11.0):** DDlog depends on a specific
  version of the FlatBuffers serialization library.

  - Download and build FlatBuffers release 1.11.0 from its official GitHub
    repository.

  - Ensure that the `flatc` compiler tool (from the FlatBuffers build) is
    accessible in your system's `$PATH`.

  - Additionally, the FlatBuffers Java classes must be available in your
    `$CLASSPATH`. The DDlog repository provides a script to help with this:
    navigate to your cloned DDlog directory and run
    `./tools/install-flatbuf.sh`. This script typically downloads and builds
    FlatBuffers, then provides export commands for `CLASSPATH` and `PATH`, such
    as:

    ```bash
    cd flatbuffers # (or the directory where install-flatbuf.sh places it)
    export CLASSPATH=`pwd`/java:$CLASSPATH
    export PATH=`pwd`:$PATH
    cd..

    ```

  .1

- **Static Libraries (Primarily for Compiling DDlog from Source):** If you
  intend to compile DDlog itself from its source code, you may need static
  versions of several standard C/C++ libraries, including `libpthread.a`,
  `libc.a`, `libgmp.a` (GNU Multiple Precision Arithmetic Library), and others.1

  - On Ubuntu, these can often be installed with:
    `sudo apt install libc6-dev libgmp-dev`.
  - On Fedora, the command would be similar to:
    `sudo dnf install glibc-static gmp-static libstdc++-static`. .1

The prerequisite list, particularly the need for a JDK and a specific version of
FlatBuffers with manual `PATH` and `CLASSPATH` configuration 1, indicates that
DDlog's setup is more involved than that of a typical Rust crate managed solely
by Cargo. The JDK and FlatBuffers Java class dependencies likely stem from
components of the DDlog compiler's toolchain or its support for generating Java
bindings.1 Careful attention to these steps is necessary to avoid installation
issues. The provided `install-flatbuf.sh` script 1 is a key utility for
simplifying the FlatBuffers setup.

### B. Installation Options

There are two primary methods for installing DDlog:

#### 1. Using a Pre-compiled Binary Release (Recommended)

This is generally the easiest and recommended method for most users.

- Navigate to the DDlog GitHub releases page:
  `https://github.com/vmware-archive/differential-datalog/releases`.6
- Download the latest binary release archive for Linux (e.g.,
  `ddlog-vX.Y.Z-...-Linux.tar.gz`).6
- Extract the archive to a suitable location (e.g., `~/ddlog` or `/opt/ddlog`).
- Add the `bin` subdirectory of your DDlog installation to your system's
  `$PATH`. For example, if you extracted DDlog to `~/ddlog`, add
  `export PATH=~/ddlog/bin:$PATH` to your shell's configuration file (e.g.,
  `~/.bashrc` or `~/.zshrc`).
- Set the `$DDLOG_HOME` environment variable to point to the root of your DDlog
  installation directory. For example: `export DDLOG_HOME=~/ddlog`. .2

The availability of pre-compiled Linux binaries 2 significantly lowers the
barrier to entry by bypassing the complex source compilation process and its
extensive dependencies. This path is the most direct way to get started.

#### 2. Compiling DDlog from Source

This option is for users who need the absolute latest (potentially unreleased)
changes or if pre-compiled binaries are unavailable or unsuitable for their
specific Linux distribution.

- Clone the DDlog repository:
  `git clone https://github.com/vmware-archive/differential-datalog.git`
- Ensure all prerequisites listed in Section II.A are met, especially JDK,
  FlatBuffers, and potentially static C libraries.1
- The DDlog repository contains a `stack.yaml` file 7, which indicates the use
  of the Haskell Stack tool for building parts of DDlog (likely the compiler
  itself). The compilation process would typically be initiated by build scripts
  or commands specific to the DDlog project (users should consult the
  `README.md` or build instructions within the cloned repository for the exact
  commands, as these are not detailed in the provided snippets for a full source
  build of DDlog itself).

### C. Verifying Your Installation

After installation (preferably via binary release), verify that DDlog is
correctly set up:

- Open a new terminal session to ensure environment variable changes (`$PATH`,
  `$DDLOG_HOME`) have taken effect.
- Run the command: `ddlog --version` This should print the installed DDlog
  version.
- Alternatively, try compiling a very simple DDlog file (see Section III.D for
  an example) using the `ddlog` command. The tutorial documentation frequently
  refers to command-line interactions 3, implying the `ddlog` executable should
  be readily available and functional after a correct installation.

### D. Setting up Vim Syntax Highlighting (Optional)

For developers using Vim or Neovim, DDlog provides syntax highlighting for `.dl`
files:

- Create a symbolic link from the `dl.vim` syntax file in your DDlog
  installation to your Vim syntax directory:

  ```bash
  # Assuming DDLOG_HOME is set
  ln -s $DDLOG_HOME/tools/vim/syntax/dl.vim ~/.vim/syntax/dl.vim

  ```

  (Adjust `~/.vim/syntax/` if using Neovim, e.g., `~/.config/nvim/syntax/`).1

- If using a Vim plugin manager like Vundle, you might add a line similar to
  this to your Vim configuration:
  `Plugin 'vmware/differential-datalog', {'rtp': 'tools/vim'}` .1

## III. Your First DDlog Program: The Basics

DDlog programs are written in files with a `.dl` extension.1 Understanding their
structure and fundamental components—types, relations, and rules—is the first
step in writing DDlog logic.

### A. DDlog Program Structure (`.dl` files)

A typical DDlog program consists of several types of declarations:

- **Data type declarations:** Defining custom types, similar to enums or
  structs.
- **Input relation declarations:** Specifying the schema of data that will be
  fed into the program.
- **Output relation declarations:** Specifying the schema of data that the
  program will compute and output.
- **Rules:** Defining the logic for how output relations are derived from input
  relations and other derived relations. .3

DDlog supports C++/Java-style comments:

- Single-line comments start with `//`.
- Multi-line comments are enclosed in `/* */`. Notably, these multi-line
  comments can be nested, which is useful for commenting out large blocks of
  code that may already contain comments.3

### B. Defining Data: Types and Relations

Data in DDlog is structured using types and stored in relations.

Types:

DDlog has a rich type system. Users can define their own types using typedef.
For example, an enumeration-like type (tagged union) can be declared as:

```prolog
typedef Category = CategoryStarWars | CategoryOther;
```

.3

There are specific capitalization conventions:

- Type names (e.g., `Category`) have no strict capitalization rules, but
  consistency is good practice.
- Type constructors (e.g., `CategoryStarWars`, `CategoryOther`) must start with
  an uppercase letter.3

Relations:

Relations in DDlog are similar to tables in a relational database; they are
collections (sets) of records. Records are like C structs, consisting of
multiple named fields, each with a specific type.3

- **Input Relations:** Declared using the `input relation` keywords, they define
  the structure of data that the DDlog program expects to receive from an
  external source.

  ```prolog
  input relation Word1(word: string, cat: Category);

  ```

- **Output Relations:** Declared using the `output relation` keywords, they
  define the structure of data that the DDlog program will compute.

  ```prolog
  output relation Phrases(phrase: string);

  ```

.3

- Relation names (e.g., `Word1`, `Phrases`) must start with an uppercase
  letter.3

The clear distinction between `input` and `output` relations 3 is fundamental to
DDlog's dataflow architecture.1 Input relations serve as the entry points for
external data, while output relations hold the results derived by DDlog's rules.
This separation underpins the reactive nature of DDlog: modifications to input
relations trigger the incremental computation process, which in turn updates the
output relations.

### C. Defining Logic: Rules and Facts

The core logic of a DDlog program is expressed through rules. A rule defines how
to derive new facts (records in relations) based on existing facts.

The basic syntax of a rule is:

Head :- Body.

- **Head:** The part of the rule to the left of the `:-` (read as "if" or "is
  implied by"). It specifies a fact to be derived.
- **Body:** The part of the rule to the right of `:-`. It consists of one or
  more *literals* (predicates) that specify conditions. If all literals in the
  body are true, the fact in the head is derived.
- Literals in the body are separated by commas (`,`), which represent a logical
  AND.3
- Rules must end with a period (`.`).3
- Variable names (e.g., `w1`, `cat` in the example below) must start with a
  lowercase letter or an underscore.3

Example rule:

```prolog
Phrases(w1 ++ " " ++ w2) :- Word1(w1, cat), Word2(w2, cat).
```

This rule states: "For every record `Word1(w1, cat)` in the `Word1` relation and
every record `Word2(w2, cat)` in the `Word2` relation (where the `cat` field is
the same in both), derive a new fact `Phrases(phrase)` where `phrase` is the
concatenation of `w1`, a space, and `w2`".3

Facts are the actual data records within relations. Some facts are provided as
input, while others are derived by rules. For instance,
`Path(x, y) :- Edge(x, y).` is a rule stating that if there's an `Edge(x, y)`
fact, then a `Path(x, y)` fact is also true.8

The `:-` operator embodies Datalog's declarative strength. It expresses logical
implication: if the conditions in the body are met by existing facts, then the
head becomes a new fact. This simple construct, especially when combined with
recursion (e.g., defining a path in terms of edges and shorter paths, as in
`Path(x,z) :- Edge(x,y), Path(y,z).` 8), allows for concise expression of
complex computations. The programmer declares the conditions for a fact's
existence, and the DDlog engine determines how to derive all such facts.

### D. A Simple "Hello, World" Style Example

The following is a small, complete DDlog program based on the tutorial
examples.3 This program can be saved as `example.dl`:

```prolog
// example.dl

/* Define a custom data type for categories */
typedef Category = CategoryStarWars | CategoryOther;

/* Define input relations: Word1 and Word2 */
// These relations expect records with a string and a Category.
input relation Word1(word: string, cat: Category);
input relation Word2(word: string, cat: Category);

/* Define an output relation: Phrases */
// This relation will contain records with a single string field.
output relation Phrases(phrase: string);

/* Define a rule to compute Phrases */
// If there's a word w1 of category cat in Word1,
// and a word w2 of the same category cat in Word2,
// then create a phrase by concatenating w1, a space, and w2.
Phrases(w1 ++ " " ++ w2) :- Word1(w1, cat), Word2(w2, cat).
```

This example illustrates type definition, input and output relation
declarations, and a simple rule that performs a join and string concatenation.
It will serve as the basis for demonstrating compilation and interaction in
subsequent sections.

## IV. Compiling and Integrating DDlog with Your Rust Project

A primary use case for DDlog is its integration into larger applications,
particularly those written in Rust. This section covers the compilation process,
how to set up a Rust project for DDlog, and how to interact with a compiled
DDlog program.

### A. The DDlog Compilation Process: From `.dl` to a Rust Library

The DDlog compiler transforms your `.dl` program into a Rust library.1 This
process involves two main stages:

1. The `ddlog` compiler takes your `.dl` file(s) as input and generates Rust
   source code (`.rs` files).
2. This generated Rust code is then compiled by the standard Rust compiler
   (`rustc`), typically via Cargo, into a library that your main Rust
   application can link against and use.3

An important aspect of this compiled approach is that any changes to the DDlog
program's relational schema (the structure of your relations) or its rules
necessitate a re-compilation of both the DDlog program into Rust code and then
the subsequent Rust compilation.1

Compilation times, especially for the Rust compilation phase of the generated
code, can sometimes be lengthy. One discussion noted that "Datalog queries get
compiled down to Rust code, and that takes minutes. Not very interactive,
unfortunately".4 To help manage this, the DDlog documentation suggests several
strategies to potentially speed up re-compilation cycles:

- **Modularize your DDlog program:** Decompose large DDlog programs into smaller
  modules that do not have circular dependencies. The DDlog compiler will then
  generate a separate Rust crate for each module, which can lead to faster
  incremental Rust builds.3
- **Preserve Rust compiler artifacts:** Avoid deleting the `playpen_ddlog`
  directory (or a similarly named directory where DDlog stages its generated
  Rust code and Cargo builds it, often within a `target` subdirectory of your
  DDlog project) between builds, unless you are upgrading the DDlog compiler
  itself. This directory contains Rust compiler artifacts that can speed up
  subsequent compilations.3
- **Adjust Rust optimization levels (with caution):**
  - For faster Rust compilation during development (at the cost of runtime
    performance), set the environment variable
    `CARGO_PROFILE_RELEASE_OPT_LEVEL="z"`. This can significantly speed up the
    Rust compilation phase, but the resulting executable may run up to 50%
    slower. This setting should be disabled for benchmarking or production
    builds.3
  - Using a Rust debug build (e.g., `cargo build` without `--release`) will
    compile the generated Rust code much faster, but the resulting binaries will
    be very large and significantly slower than release builds.3

### B. Creating a New Rust Project for DDlog Integration

To use a DDlog program within a Rust application, start by creating a new Rust
project using Cargo:

```bash
cargo new my_ddlog_app --bin
cd my_ddlog_app
```

You will typically place your DDlog program (e.g., `my_program.dl`) within this
project structure, perhaps in a dedicated subdirectory (e.g., `ddlog_src/`).

The integration often involves a `build.rs` script in your Rust project's root
directory. This script will be responsible for invoking the DDlog compiler on
your `.dl` file(s) during the build process of your Rust application. The
`build.rs` script tells Cargo how to compile and link native libraries or, in
this case, how to generate Rust code from another language (DDlog) and make it
available to the main Rust crate.

### C. Linking the Compiled DDlog Library

The `build.rs` script typically handles the DDlog compilation and informs Cargo
about the generated Rust code's location (usually in `OUT_DIR`). Your main Rust
project's `Cargo.toml` will then need to declare a dependency on the crate
generated by DDlog.

A simplified conceptual `build.rs` might look like this (actual implementation
details can vary based on DDlog version and helper crates):

```rust
// build.rs (conceptual)
use std::process::Command;
use std::env;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let ddlog_program_path = "ddlog_src/my_program.dl"; // Path to your.dl file
    let crate_name = "my_program_ddlog"; // Name for the generated crate

    println!("cargo:rerun-if-changed={}", ddlog_program_path);

    // Invoke the ddlog compiler
    let status = Command::new("ddlog")
       .arg("-i")
       .arg(ddlog_program_path)
       .arg("-L") // Specify output directory for the Rust crate
       .arg(Path::new(&out_dir).join(crate_name).to_str().unwrap())
       .arg("--crate-name")
       .arg(crate_name)
       .status()
       .expect("Failed to execute ddlog compiler");

    if!status.success() {
        panic!("ddlog compilation failed");
    }

    // Tell Cargo where to find the generated crate
    println!(
        "cargo:rustc-link-search=native={}",
        Path::new(&out_dir)
            .join(crate_name)
            .join("target/debug")
            .display(),
    ); // Adjust for release
    println!("cargo:rustc-link-lib=dylib={}", crate_name); // Or static, depending on DDlog output
}
```

**Note:** The exact command-line arguments for `ddlog` and how the generated
library is structured and linked can vary. Consult the DDlog documentation
specific to the version you are using for precise `build.rs` instructions.
Often, DDlog provides helper Rust crates to manage this process more smoothly.

In your `Cargo.toml`, you would then reference the generated types and
functions.

### D. Using the Generated Rust API: An Overview

Once a DDlog program is compiled into a Rust library, it exposes a Rust API that
your application can use to interact with it. This API typically allows you to:

- **Instantiate and start** the DDlog program.
- **Begin transactions.**
- **Insert, delete, or modify records** in the DDlog program's input relations.
- **Commit transactions,** triggering incremental computation.
- **Query or receive updates** from the DDlog program's output relations.

The specifics of this API (function names, data structures) are derived from
your DDlog program's relation and type definitions. For example, if you have an
input relation `MyInput(field1: string, field2: u32)`, the generated API might
include a function like
`insert_MyInput(&mut self, field1: String, field2: u32) -> Result<(), String>`.

### E. Running and Interacting via the Command-Line Interface (CLI)

While the primary goal is often library integration, DDlog also supports running
compiled programs via a command-line interface.1 The DDlog compiler can produce
an executable that provides a text-based interface to interact with your DDlog
logic.3 This CLI is invaluable for:

- Testing DDlog programs in isolation.
- Debugging rules and data transformations.
- Understanding the incremental behavior of your program.

To compile your `example.dl` (from Section III.D) for CLI use, you would
typically run:

```bash
ddlog -i example.dl -o example_cli
# This creates a Rust project for the CLI in a directory like example_cli_ddlog/
cd example_cli_ddlog
cargo build --release
# The executable will be in target/release/example_cli
./target/release/example_cli
```

Once the CLI is running (it will show a `>>` prompt), you can interact with it
using commands. For instance, to interact with the `example.dl` program:

```text
>> start;
>> insert Word1("Hello,", CategoryOther);
>> insert Word2("world!", CategoryOther);
>> commit dump_changes;
Phrases:
Phrases{.phrase = "Hello, world!"}: +1
>> dump Phrases;
Phrases:
Phrases{.phrase = "Hello, world!"}
>> exit;
```

.3

The CLI's transaction model (`start`, `insert`, `commit dump_changes` 3) closely
mirrors how a Rust application would programmatically interact with the DDlog
library. The `dump_changes` part of the commit command is particularly
insightful, as it reveals only the incremental changes to the output relations,
which is a core aspect of DDlog's behavior. Mastering these CLI interaction
patterns provides a solid foundation for understanding and implementing the
programmatic Rust API interactions.

The following table summarizes common DDlog CLI commands:

| Command                                              | Description                                                                                                                               |
| ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| start;                                               | Begins a new transaction. All subsequent data modifications (inserts, deletes) occur within this transaction.                             |
| insert RelationName(field1_value, field2_value,...); | Inserts a new record into the specified input relation. Multiple insert commands can be part of a single transaction.                     |
| delete RelationName(field1_value, field2_value,...); | Deletes an existing record from the specified input relation.                                                                             |
| commit dump_changes;                                 | Commits the current transaction, applies all changes, and displays the incremental changes (additions/deletions) to the output relations. |
| commit;                                              | Commits the current transaction and applies changes without displaying them.                                                              |
| dump RelationName;                                   | Displays all current records in the specified relation (can be an input or output relation).                                              |
| exit; or quit;                                       | Exits the DDlog command-line interface.                                                                                                   |

This table provides a quick reference for experimenting with DDlog programs via
the CLI, facilitating easier learning and testing.

## V. Working with DDlog: Essential Concepts for Rust Developers

To effectively use DDlog with Rust, developers should be familiar with several
core DDlog concepts that influence how programs are written and how they
interact with Rust code.

### A. Data Types in Depth

DDlog features a comprehensive type system that allows for precise data
modeling.1 This is a significant advantage, as it enables developers to
represent complex, structured data directly within DDlog relations, often
avoiding the normalization or flattening required in traditional Datalog or SQL.
This aligns well with Rust's own strong, static type system and its common usage
of rich collection types.

Key DDlog data types include:

- **Primitive Types:** Booleans (`bool`), unlimited precision integers
  (`bigint`), fixed-size signed integers (`signed<N>`, where `N` is the number
  of bits), fixed-size unsigned integers/bitvectors (`bit<N>`), and
  floating-point numbers (`float`, `double`).
- **String Type:** `string` for Unicode text.
- **Tuples:** Fixed-size, ordered collections of potentially heterogeneous
  values, e.g., `(string, u32)`.
- **Tagged Unions (Enums):** Defined with `typedef`, e.g.,
  `typedef MyEnum = VariantA | VariantB(u32)`.
- **Collection Types:**
  - `Vec<T>`: Dynamically-sized, ordered vectors (lists).
  - `Set<T>`: Unordered collections of unique elements.
  - `Map<K,V>`: Collections of key-value pairs. .1

DDlog also supports various **type conversions**, such as between different
bit-width integers (`bit<N>` to `bit<M>`, `signed<N>` to `signed<M>`), between
signed and unsigned integers (`bit<N>` to `signed<N>`), and from fixed-width
integers to arbitrary-precision integers (`bit<N>` to `bigint`).3

The following table maps DDlog types to their conceptual Rust equivalents,
aiding developers familiar with Rust:

| DDlog Type     | Potential Rust Equivalent (Conceptual) | Description                                                          | Snippet Refs         |
| -------------- | -------------------------------------- | -------------------------------------------------------------------- | -------------------- |
| bool           | bool                                   | Boolean value (true or false).                                       | 1                    |
| bigint         | num_bigint::BigInt (via library)       | Unlimited precision integer.                                         | 1                    |
| `bit<N>`       | u8, u16, u32, u64, u128                | N-bit unsigned integer (bitvector).                                  | 1                    |
| `signed<N>`    | i8, i16, i32, i64, i128                | N-bit signed integer.                                                | 1                    |
| float          | f32                                    | Single-precision floating point number.                              | 1                    |
| double         | f64                                    | Double-precision floating point number.                              | 1 (implied)          |
| string         | String                                 | Unicode string.                                                      | 1                    |
| `(T1, T2,...)` | (T1, T2,...) (Rust tuple)              | Fixed-size collection of heterogeneous values (DDlog syntax varies). | 1                    |
| `typedef T = A | B`                                     | enum T { A, B }                                                      | Tagged union (enum). |
| `Vec<T>`       | `Vec<T>`                               | Dynamically-sized vector (ordered list).                             | 1                    |
| `Set<T>`       | `HashSet<T>`                           | Unordered collection of unique elements.                             | 1                    |
| `Map<K,V>`     | `HashMap<K,V>`                         | Collection of key-value pairs.                                       | 1                    |

### B. Writing Expressive Rules

Rules are the heart of DDlog logic. Beyond simple fact derivation, rules can
express complex relationships:

- **Joins:** Achieved by using shared variables across multiple literals in a
  rule's body. For example:
  `OutputRel(x, z) :- InputRel1(x, y), InputRel2(y, z).` Here, the variable `y`
  joins records from `InputRel1` and `InputRel2` where their respective fields
  match. This is implicit in examples like the `Phrases` rule.3
- **Conditions/Filters:** Boolean expressions can be added to literals to filter
  records. For example: `Eligible(person) :- Person(person, age), age >= 18.`
- **Assignments within Rules:** DDlog allows assignments using `let` or by
  binding variables in the head of a rule based on expressions. For example, in
  `HostIP(host, addr) :- HostAddress(host, addrs), var addr =`
  `split_ip_list(addrs).flat_map(|a| Some(a))` 3, `addr` is bound by iterating
  over the result of a function call.
- **Procedural Constructs:** While DDlog is primarily declarative, it
  incorporates procedural elements to enhance expressiveness, especially within
  functions (see next section). These include sequential execution (using
  semicolons), `if/else` statements, `match` expressions (similar to Rust's
  `match` or C/Java `switch`), `for` loops, `continue`, `break`, and `return`.3
- **Aggregates:** Use aggregate functions like `sum`, `count`, or `max` to
  compute values grouped by the variables outside the aggregate expression.
  Example: `Totals(x, sum y) :- Values(x, y).`

### C. Using Functions in DDlog

DDlog allows the definition and use of functions, which are similar to
user-defined functions (UDFs) in relational database systems. Functions can
encapsulate imperative logic, including control flow constructs and operations
on complex data types. They are designed to operate on values that typically
originate from relations.1

For example, a rule might use a function to process a field:

ProcessedData(id, result) :- RawData(id, value), var result =
my_processing_function(value).

The function split_ip_list() is used in tutorial examples.3

DDlog supports **Universal Function Call Syntax (UFCS)**, meaning a function
call like `average(my_set_of_scores)` can also be written as
`my_set_of_scores.average()`.8 This syntax is familiar to Rust developers.

The inclusion of functions with imperative capabilities 1 represents a pragmatic
extension to pure Datalog. It acknowledges that not all computations fit neatly
into a purely declarative rule-based format. This allows DDlog to tackle a
broader spectrum of problems more effectively within the language itself,
reducing the need for extensive external pre- or post-processing of data. This
design balances declarative elegance with practical expressive power.

### D. Transactions

Atomicity and consistency are managed in DDlog through transactions:

- All data modification operations (insertions, deletions) on input relations
  must be part of a transaction.3
- A transaction is initiated with the `start;` command (in the CLI).
- Changes are applied and made visible (and incremental updates computed) when
  the transaction is committed, typically using `commit dump_changes;` (in the
  CLI, to also see the changes) or just `commit;`.3
- By default, relations declared with the `relation` keyword are **sets**,
  meaning they cannot contain multiple identical records. If an attempt is made
  to insert an existing record, the relation remains unchanged.3 The
  documentation mentions that relations with multiset semantics (allowing
  duplicates) can also be introduced.3

### E. String Manipulation

DDlog provides built-in support for string manipulation:

- **Concatenation:** Strings can be concatenated using the `++` operator, e.g.,
  `w1 ++ " " ++ w2`.1
- **Interpolation:** String interpolation is also supported, allowing
  expressions to be embedded within strings.1
- **String Constants:** DDlog supports two forms of string literals:
  1. **Quoted strings:** Enclosed in double quotes (`"`), they support C-style
     escape sequences (e.g., ` `, `\t`, `\"`, `\\`). Long strings can be broken
     across multiple lines using a backslash (`\`) at the end of a line; the
     backslash and any immediately following whitespace are ignored.3 Example:
     `"Hello\tworld"`.
  2. **Raw strings:** Enclosed in `[|` and `|]`, they can contain arbitrary
     Unicode characters, including newlines and backslashes, without escaping,
     except for the `|]` sequence itself.3 Example:
     `[|A raw string with a \ (backslash) and a newline.|]`.

## VI. Practical Example: Building a Small Application with DDlog and Rust

To solidify understanding, this section walks through creating a simple
application that uses DDlog for logic and Rust as the host language. We will
implement a basic graph reachability analysis.

### A. Defining a Problem

The problem is to find all pairs of nodes (u,v) in a directed graph such that
there is a path from node u to node v. This is a classic Datalog example.8

- **Input:** A set of directed edges, where each edge is a pair of node
  identifiers (e.g., `u32`).
- **Output:** A set of pairs (u,v) representing that node v is reachable from
  node u.

We will define:

- An input relation `Edge(from: u32, to: u32)` to represent the graph's edges.
- An output relation `Reachable(from: u32, to: u32)` to store the computed
  reachability pairs.

### B. Writing the DDlog Program (`reachability.dl`)

Create a file named `reachability.dl` with the following content:

```prolog
// reachability.dl

// Input relation: represents directed edges in a graph.
// 'from' is the source node, 'to' is the destination node.
input relation Edge(from: u32, to: u32);

// Output relation: represents pairs of nodes (from, to)
// such that 'to' is reachable from 'from'.
output relation Reachable(from: u32, to: u32);

// Rule 1: Base case for reachability.
// An edge from x to y directly implies that y is reachable from x.
Reachable(x, y) :- Edge(x, y).

// Rule 2: Recursive case for reachability.
// If there is an edge from x to y, AND y can reach z,
// then x can also reach z.
Reachable(x, z) :- Edge(x, y), Reachable(y, z).
```

This program defines the input `Edge` relation and the output `Reachable`
relation. The first rule establishes direct reachability from an edge. The
second rule is recursive: if there's an edge from `x` to `y`, and `y` can
already reach `z` (due to previous applications of these rules), then `x` can
also reach `z`.

### C. Writing the Rust Host Program (`main.rs`)

Now, let's outline the Rust program (`src/main.rs`) that will use this DDlog
program. This involves initializing the DDlog engine, feeding it `Edge` facts,
committing changes, and then observing the `Reachable` facts.

**Note:** The exact Rust API for interacting with the compiled DDlog library
depends on the DDlog version and any helper crates it provides. The following
Rust code is conceptual and illustrative of the general interaction patterns.
You would typically use a `build.rs` script (as discussed in Section IV.C) to
compile `reachability.dl` into a Rust crate (e.g., named `reachability_ddlog`)
that `main.rs` can then use.

```rust
// src/main.rs (Conceptual - actual API may vary)

// Assume `reachability_ddlog` is the crate generated from reachability.dl
// and it provides types like `Edge` and functions to interact.
// It might also provide a top-level struct or functions like `HDDlog::run`.

// This line would typically be generated or provided by the DDlog Rust bindings.
// It imports necessary items from the DDlog-generated crate.
// For example:
// extern crate reachability_ddlog;
// use reachability_ddlog::{HDDlog, Relations, Record, upd_cb_t}; // Example types

use std::sync::{Arc, Mutex};

// Callback function to handle changes in the Reachable relation
fn print_reachable_updates(relation_id: Relations, rec: &Record, weight: isize) {
    // Relations::Reachable would be an enum variant identifying the Reachable relation
    if relation_id == Relations::Reachable { // This check needs actual enum from generated code
        if let Record::Reachable { from, to } = rec { // Destructure based on generated Record enum
            if weight > 0 {
                println!("New reachable path: {} -> {} (count: {})", from, to, weight);
            } else {
                println!("Reachable path removed: {} -> {} (count: {})", from, to, weight);
            }
        }
    }
}

fn main() -> Result<(), String> {
    // 1. Initializing the DDlog program
    // The `HDDlog::run` function typically takes the number of worker threads
    // and a callback for handling output relation updates.
    // The callback mechanism is crucial for observing incremental changes.
    // [10] shows `inspect(|x| println!("observed: {:?}", x)).probe()` for differential-dataflow,
    // suggesting a callback/streaming model.

    // The `Arc<Mutex<()>>` is a placeholder for actual shared state if needed by callbacks.
    // The callback `print_reachable_updates` will be invoked by DDlog when `Reachable` changes.
    // The exact signature of HDDlog::run and the callback will come from the generated bindings.
    // let (mut ddlog_prog, _ddlog_delta_reader) = HDDlog::run(1, true, print_reachable_updates)?;
    
    // Placeholder for actual DDlog program handle.
    // The API to start, transact, and stop would be called on this handle.
    // For this example, let's imagine a simplified API.
    // let mut ddlog_api = MyDdlogAPI::new()?; // Conceptual API handle

    println!("DDlog program initialized.");

    // 2. Feeding input data (inserting facts)
    // Operations are typically grouped into transactions.
    // ddlog_api.transaction_start()?;
    println!("Starting transaction 1...");

    // ddlog_api.insert_edge(0, 1)?;
    // ddlog_api.insert_edge(1, 2)?;
    // ddlog_api.insert_edge(2, 3)?;
    println!("Inserted edges: 0->1, 1->2, 2->3");

    // ddlog_api.transaction_commit_dump_changes()?;
    println!("Committed transaction 1.");

    // At this point, the `print_reachable_updates` callback would have been invoked
    // for newly derived Reachable facts.

    // 3. Handling updates incrementally
    // ddlog_api.transaction_start()?;
    println!("
Starting transaction 2...");

    // ddlog_api.insert_edge(3, 0)?; // Introduce a cycle
    // ddlog_api.insert_edge(0, 4)?; // Add a new branch
    println!("Inserted edges: 3->0 (cycle), 0->4");

    // ddlog_api.transaction_commit_dump_changes()?;
    println!("Committed transaction 2.");
    
    // The callback would again show new Reachable paths, including those
    // resulting from the cycle and the new branch, computed incrementally.

    // 4. Querying output relations (alternative to callbacks, if available)
    // Some APIs might allow dumping the current state of a relation.
    // let current_reachable_paths = ddlog_api.dump_reachable()?;
    // println!("
Current Reachable paths (full dump): {:?}", current_reachable_paths);

    // ddlog_api.stop()?;
    println!("
DDlog program stopped.");

    Ok(())
}
```

The interaction between Rust and the DDlog-generated library centers on an API
for managing data transactions and observing output changes. The
`commit dump_changes` behavior seen in the CLI 3 suggests that the Rust API will
likely provide mechanisms to apply batches of input changes (inserts/deletes)
and, crucially, to receive batches of *output changes* (deltas indicating what
was added or removed from output relations). This delta-based output is
fundamental to leveraging DDlog's incremental computation capabilities
efficiently from Rust. The callback mechanism (like `print_reachable_updates`)
is a common pattern for handling such streams of changes.

### D. Compiling and Running the Integrated Application

Assuming you have:

1. `reachability.dl` in a `ddlog_src/` directory.
2. A `build.rs` script correctly configured to compile `reachability.dl` using
   the `ddlog` command and link the resulting `reachability_ddlog` crate.
3. The conceptual `src/main.rs` (adapted with the actual generated API).
4. Dependencies (like the DDlog runtime crate) specified in `Cargo.toml`.

You would compile and run your application using standard Cargo commands:

```bash
cargo build --release
cargo run --release
```

The output would show the initial `Reachable` paths derived after the first
transaction, followed by additional `Reachable` paths derived incrementally
after the second transaction, all printed by the callback function.

## VII. Further Considerations

Beyond the basics, several other aspects are relevant when working with DDlog.

### A. Performance Characteristics

- **Incremental Computation:** DDlog's core strength is its ability to minimize
  redundant work by only computing changes to output relations based on input
  updates.2 This is achieved through its foundation in differential dataflow, a
  framework designed for data-parallel processing and incremental updates.8
- **Compilation Time vs. Runtime Performance:**
  - As noted earlier, compiling DDlog programs into Rust, and then compiling
    that Rust code, can be time-consuming, especially for larger DDlog
    programs.4
  - The runtime performance of the generated code is generally good due to
    compilation to Rust. However, development-time optimizations for faster Rust
    compilation (like `CARGO_PROFILE_RELEASE_OPT_LEVEL="z"`) can lead to slower
    runtime execution (potentially 50% slower).3 A balance must be struck based
    on development phase versus production deployment.

### B. Debugging Tips

Debugging DDlog programs can involve several strategies:

- **Use the CLI:** The command-line interface is an excellent tool for testing
  DDlog logic in isolation. Insert facts step-by-step and use
  `dump RelationName;` to inspect the contents of input and output relations at
  various stages.
- **Simplify Rules:** If encountering unexpected behavior, try commenting out or
  simplifying complex rules to isolate the source of the issue.
- **Incremental** `commit dump_changes;`**:** In the CLI, use
  `commit dump_changes;` frequently to observe how each small set of input
  changes affects the output relations. This helps understand the flow of data
  and derivation.
- **Self-Profiler:** DDlog includes a self-profiling capability that can
  generate interactive HTML reports detailing operator performance, memory
  usage, and links to source code locations.6 This is a more advanced tool that
  can help understand performance bottlenecks or unexpected computational effort
  in complex programs.

### C. Understanding Output Changes

When DDlog (especially via the CLI's `commit dump_changes;` command or a Rust
callback API) reports changes to output relations, it typically indicates not
just the record itself but also its change in multiplicity (weight). For set
relations (the default), this is usually:

- `RelationName{...fields...}: +1` indicates that the record was inserted into
  the relation.3
- `RelationName{...fields...}: -1` (if deletions were supported and occurred)
  would indicate that the record was deleted from the relation. This `+/-1`
  notation is how DDlog communicates incremental updates.

## VIII. Conclusion and Resources

Differential Datalog, when combined with Rust on Linux, provides a robust
framework for building applications that require efficient, incremental
processing of relational data. Its declarative nature simplifies the expression
of complex data dependencies and transformations, while its compilation to Rust
ensures good performance and seamless integration into the Rust ecosystem.

### A. Recap of DDlog's Strengths for Rust on Linux

- **Declarative Power:** Define *what* data relationships and derivations are
  needed, letting DDlog manage the *how* of incremental computation.
- **Incremental Efficiency:** Process input changes with minimal computational
  overhead, crucial for dynamic data environments.
- **Strong Typing:** Leverage a rich type system for safe and expressive data
  modeling, aligning well with Rust's own type safety.
- **Rust Integration:** Compile DDlog logic directly into Rust libraries for
  tight coupling, good performance, and access to the Rust ecosystem.
- **Versatility:** Suitable for a range of applications, from real-time
  analytics and network monitoring to static program analysis.

While the `vmware-archive/differential-datalog` repository is archived, the
existing codebase and documentation provide a solid foundation for developers
looking to explore and utilize incremental computation.

### B. Pointers to Official Documentation and Community

For further exploration and detailed information, refer to the following
resources, keeping in mind the archived status of the primary repository:

- **Main GitHub Repository:**
  `https://github.com/vmware-archive/differential-datalog`.1 This is the source
  for the code and original documentation.
- **DDlog Tutorial:** The most comprehensive guide for learning DDlog syntax and
  concepts is likely the `tutorial.md` file found within the repository's
  `doc/tutorial/` directory.3
  - Direct link (may depend on commit history):
    `https://github.com/vmware-archive/differential-datalog/blob/master/doc/tutorial/tutorial.md`
- **Examples:** The tutorial mentions that examples can be found in
  `test/datalog_tests/tutorial.dl` within the repository, along with test inputs
  (`.dat` files) and expected outputs (`.dump.expected` files).3 Exploring these
  can provide practical insights into DDlog usage.
- **Differential Dataflow:** For a deeper understanding of the underlying
  framework, exploring documentation related to Differential Dataflow (by Frank
  McSherry) can be beneficial, as DDlog is based on it.1

By carefully following the setup instructions, understanding the core Datalog
principles extended by DDlog, and leveraging the Rust integration pathway,
developers can effectively harness the power of incremental computation for
their Linux-based applications.
