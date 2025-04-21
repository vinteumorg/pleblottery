{
  description = "pleblottery development environment with SV2 Bitcoin Core";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        
        # Rust toolchain
        rustToolchain = pkgs.rust-bin.stable.latest.default;
        
        # Environment variables
        bitcoinDir = "/tmp/pleblottery/.bitcoin";
        bitcoinConf = "${bitcoinDir}/bitcoin.conf";
        rpcPort = 38332; # signet
        
        # Script to start services in tmux with side-by-side panes
        pleblotteryPlaygroundScript = pkgs.writeShellScriptBin "pleblottery_playground" ''
          # Check if the RPC port is already in use
          if ${pkgs.nmap}/bin/nmap -p ${toString rpcPort} localhost | grep -q "^${toString rpcPort}/tcp.*open"; then
            echo "Error: Port ${toString rpcPort} is already in use!"
            echo "Please stop any running Bitcoin node or change the RPC port configuration."
            exit 1
          fi

          # Start tmux session with bitcoind
          ${pkgs.tmux}/bin/tmux new-session -d -s pleblottery "$BITCOIND_PATH -datadir=$BITCOIN_DATADIR -signet -sv2 -sv2port=8442"
          
          # Split the window horizontally (side-by-side)
          ${pkgs.tmux}/bin/tmux split-window -h -t pleblottery
          
          # Run pleblottery in the right pane from the repository directory
          ${pkgs.tmux}/bin/tmux send-keys -t pleblottery "cd $REPO_DIR && sleep 5 && cargo run -- -c config.toml" C-m
          
          # Set window title
          ${pkgs.tmux}/bin/tmux rename-window -t pleblottery "bitcoind | pleblottery"
          
          # Attach to the tmux session
          ${pkgs.tmux}/bin/tmux attach-session -t pleblottery
        '';
        
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Basic tools
            git
            tmux
            nmap
            
            # Nix tools
            nix
            
            # Rust
            rustToolchain
            
            # Playground script
            pleblotteryPlaygroundScript
          ];
          
          shellHook = ''
            echo "pleblottery development environment"
            
            # Check if the RPC port is already in use
            if ${pkgs.nmap}/bin/nmap -p ${toString rpcPort} localhost | grep -q "^${toString rpcPort}/tcp.*open"; then
              echo "Error: Port ${toString rpcPort} is already in use!"
              echo "Please stop any running Bitcoin node or change the RPC port configuration."
              exit 1
            fi
            
            # Store the repository directory (where flake.nix is)
            export REPO_DIR=$(pwd)
            
            # Create directories
            mkdir -p ${bitcoinDir}
            
            # Clone the repository
            cd /tmp
            if [ ! -d "nix-bitcoin-core-archive" ]; then
              echo "Cloning nix-bitcoin-core-archive..."
              ${pkgs.git}/bin/git clone https://github.com/plebhash/nix-bitcoin-core-archive
            fi
            
            # Build Sv2 fork
            echo "Building Bitcoin Core Sv2 fork by @Sjors via nix-bitcoin-core-archive..."
            cd /tmp/nix-bitcoin-core-archive/forks/sv2
            ${pkgs.nix}/bin/nix-build
            
            # Create bitcoin.conf
            echo "Creating bitcoin.conf..."
            cat > ${bitcoinConf} << EOF
            [signet]
            signetchallenge=51      # OP_TRUE
            connect=75.119.150.111  # Genesis node
            EOF
            
            # Set environment variables
            export BITCOIND_PATH=/tmp/nix-bitcoin-core-archive/forks/sv2/result/bin/bitcoind
            export BITCOIN_CLI_PATH=/tmp/nix-bitcoin-core-archive/forks/sv2/result/bin/bitcoin-cli
            export BITCOIN_DATADIR=${bitcoinDir}
            export RPC_PORT=${toString rpcPort}
            
            # Create a default config.toml in the repository directory if it doesn't exist
            if [ ! -f "$REPO_DIR/config.toml" ]; then
              echo "Creating default config.toml in the repository directory..."
              cat > "$REPO_DIR/config.toml" << EOF
            # Default pleblottery configuration
            [bitcoin]
            network = "signet"
            rpc_host = "127.0.0.1"
            rpc_port = ${toString rpcPort}
            rpc_user = "bitcoin"
            rpc_password = "bitcoin"
            EOF
            fi
            
            echo ""
            echo "Environment setup complete!"
            echo "Run 'pleblottery_playground' to launch bitcoind and pleblottery side-by-side in tmux"
            
            # Return to the repository directory
            cd "$REPO_DIR"
          '';
        };
      }
    );
} 