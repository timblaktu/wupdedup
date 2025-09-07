{
  description = "wupdedup-rs - A high-performance file deduplication and multicloud storage management tool";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    
    # Modern Rust toolchain management
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    
    # Advanced Rust build system with incremental builds
    crane = {
      url = "github:ipetkov/crane";
    };
    
    # Utilities for multi-system builds
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, crane, fenix, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        
        # Use stable Rust toolchain via Fenix
        toolchain = fenix.packages.${system}.stable.toolchain;
        
        # Create crane library with our toolchain
        craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

        # Common build arguments shared across all derivations
        commonArgs = {
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;

          # Build-time dependencies
          nativeBuildInputs = with pkgs; [
            pkg-config  # Required for OpenSSL detection
          ];

          # Runtime dependencies and system libraries
          buildInputs = with pkgs; [
            openssl  # Required by reqwest default features and object_store
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            # macOS-specific dependencies
            pkgs.libiconv
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          ];

          # Essential environment variables for OpenSSL detection
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
          
          # Performance optimization for parallel builds
          CARGO_BUILD_JOBS = toString (if pkgs.stdenv.isDarwin then 8 else 16);
          
          # Enforce warning-free builds (will fail on any warnings)
          RUSTFLAGS = "-D warnings";
        };

        # Build just the dependencies (for faster incremental builds)
        cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
          pname = "wupdedup-deps";
        });

        # Build the main application
        wupdedup-rs = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          pname = "wupdedup-rs";
          version = "0.1.0";
          
          # Set the nix shell name for proper prompt detection
          name = "wupdedup";
          
          # Additional metadata
          meta = with pkgs.lib; {
            description = "A high-performance file deduplication and multicloud storage management tool";
            homepage = "https://github.com/timblaktu/wupdedup";
            license = licenses.mit;
            maintainers = [ "Tim Blaktu" ];
            platforms = platforms.unix;
          };
        });

        # Clippy checks (runs clippy on the codebase)
        wupdedup-clippy = craneLib.cargoClippy (commonArgs // {
          inherit cargoArtifacts;
          cargoClippyExtraArgs = "--all-targets --all-features -- --deny warnings";
        });

        # Documentation generation
        wupdedup-doc = craneLib.cargoDoc (commonArgs // {
          inherit cargoArtifacts;
        });

        # Run tests with nextest (faster parallel test runner)
        wupdedup-nextest = craneLib.cargoNextest (commonArgs // {
          inherit cargoArtifacts;
          partitions = 1;
          partitionType = "count";
          cargoNextestExtraArgs = "--all-features";
        });

        # Formatting check
        wupdedup-fmt = craneLib.cargoFmt {
          inherit (commonArgs) src;
        };
        
      in
      {
        packages = {
          default = wupdedup-rs;
          wupdedup-rs = wupdedup-rs;
        };

        # All checks that run in CI
        checks = {
          inherit 
            wupdedup-rs 
            wupdedup-clippy 
            wupdedup-doc 
            wupdedup-nextest 
            wupdedup-fmt;
        };

        # Comprehensive development shell
        devShells.default = craneLib.devShell {
          # Inherit all build inputs from the package
          inputsFrom = [ wupdedup-rs ];
          
          # Set shell name for prompt detection
          name = "wupdedup";
          
          # Additional development tools
          packages = with pkgs; [
            # Enhanced development workflow
            cargo-watch        # Auto-rebuild on file changes
            cargo-edit         # cargo add/rm/upgrade commands
            cargo-expand       # View macro expansions
            cargo-audit        # Security vulnerability scanning
            cargo-outdated     # Check for outdated dependencies
            
            # Testing and benchmarking
            cargo-nextest      # Faster test runner
            criterion          # Already in Cargo.toml but useful as standalone tool
            
            # Code quality and analysis
            rust-analyzer      # IDE language server
            rustfmt           # Code formatting
            clippy            # Linting
            
            # Performance profiling (relevant for file deduplication)
            valgrind          # Memory debugging
            perf-tools        # CPU profiling
            hyperfine         # Command-line benchmarking
            
            # Development utilities
            just              # Command runner (if using justfile)
            direnv            # Automatic environment loading
            
            # Optional: Database tools (for redb inspection)
            sqlite            # In case you need DB compatibility tools
          ];

          # Development environment variables
          shellHook = ''
            # Set up wupdedup-specific environment
            export name="wupdedup"
            export NIX_SHELL_NAME="wupdedup"
            
            echo "🔨 Entering wupdedup-rs development environment"
            echo "📦 Rust toolchain: $(rustc --version)"
            echo "🔗 OpenSSL: ${pkgs.openssl.version}"
            echo "🛠️  Available tools: cargo-watch, cargo-nextest, rust-analyzer, and more"
            echo "🎨 Prompt: Your existing zsh config will show [nix:wupdedup] indicator"
            echo ""
            echo "Quick commands:"
            echo "  cargo watch -x check    # Auto-check on file changes"
            echo "  cargo nextest run       # Fast parallel testing" 
            echo "  cargo bench             # Run benchmarks"
            echo "  nix flake check         # Run all CI checks"
            echo ""
          '';

          # Development-specific environment variables
          RUST_BACKTRACE = "1";
          RUST_LOG = "debug";
          
          # Provide rust source for IDE navigation
          RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";
          
          # Optimize for development builds
          CARGO_PROFILE_DEV_DEBUG = "1";  # Include debug symbols
          CARGO_INCREMENTAL = "1";        # Enable incremental compilation
          
          # Test configuration
          RUST_TEST_THREADS = "1";        # Avoid parallel test conflicts if needed
          
          # Benchmarking configuration
          CRITERION_HOME = "./.criterion"; # Store benchmark results locally
          
          # Override RUSTFLAGS for development (allow warnings during development)
          RUSTFLAGS = "-D unused-imports -D unused-variables"; # Still deny some critical warnings
        };

        # Minimal shell for CI environments
        devShells.ci = craneLib.devShell {
          inputsFrom = [ wupdedup-rs ];
          name = "wupdedup-ci";
          packages = with pkgs; [
            cargo-nextest
            cargo-audit
          ];
        };

        # Performance testing shell with additional profiling tools
        devShells.perf = craneLib.devShell {
          inputsFrom = [ wupdedup-rs ];
          name = "wupdedup-perf";
          packages = with pkgs; [
            cargo-nextest
            valgrind
            perf-tools
            hyperfine
            flamegraph
            cargo-flamegraph
          ];
          
          shellHook = ''
            export name="wupdedup-perf"
            echo "🚀 Performance testing environment loaded"
            echo "Available profiling tools: valgrind, perf, flamegraph, hyperfine"
          '';
        };
      });
}