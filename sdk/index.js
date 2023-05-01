const path = require("path");
const { Solita } = require("@metaplex-foundation/solita");
const fs = require("fs");
const {
  rustbinMatch,
  confirmAutoMessageConsole,
} = require("@metaplex-foundation/rustbin");
const { spawn } = require("child_process");

const programDir = path.join(__dirname, "..");
const cargoToml = path.join(programDir, "Cargo.toml");
const generatedIdlDir = path.join(__dirname, "idl");
const generatedSDKDir = path.join(__dirname, "src", "generated");
const rootDir = path.join(__dirname, ".crates");

const PROGRAM_NAME = "phoenix_seat_manager";
const rustbinConfig = {
  rootDir,
  binaryName: "shank",
  binaryCrateName: "shank-cli",
  libName: "shank",
  dryRun: false,
  cargoToml,
};

async function main() {
  const { fullPathToBinary: shankExecutable } = await rustbinMatch(
    rustbinConfig,
    confirmAutoMessageConsole
  );
  const shank = spawn(shankExecutable, [
    "idl",
    "--out-dir",
    generatedIdlDir,
    "--crate-root",
    programDir,
  ])
    .on("error", (err) => {
      console.error(err);
      if (err.code === "ENOENT") {
        console.error(
          "Ensure that `shank` is installed and in your path, see:\n  https://github.com/metaplex-foundation/shank\n"
        );
      }
      process.exit(1);
    })
    .on("exit", () => {
      mutateIdl();
      generateTypeScriptSDK().then(() => {
        console.log("Running prettier on generated files...");
        // Note: prettier is not a dependency of this package, so it must be installed
        // TODO: Add a prettier config file for consistent style
        spawn("prettier", ["--write", generatedSDKDir], {
          stdio: "inherit",
        })
          .on("error", (err) => {
            console.log(err);
            console.error(
              "Failed to lint client files. Try installing prettier (`npm install --save-dev --save-exact prettier`)"
            );
          })
          .on("exit", () => {
            console.log("Finished linting files.");
          });
      });
    });

  shank.stdout.on("data", (buf) => console.log(buf.toString("utf8")));
  shank.stderr.on("data", (buf) => console.error(buf.toString("utf8")));
}

function mutateIdl() {
  console.error("Mutating IDL");
  const generatedIdlPath = path.join(generatedIdlDir, `${PROGRAM_NAME}.json`);
  const idl = require(generatedIdlPath);
  for (const instruction of idl.instructions) {
    if (instruction.name === "ChangeMarketStatus") {
      instruction.args.push({
        name: "marketStatus",
        type: {
          defined: "MarketStatus",
        },
      });
    }
    if (instruction.name === "NameMarketAuthoritySuccessor") {
      instruction.args.push({
        name: "successor",
        type: "publicKey",
      });
    }
  }
  fs.writeFileSync(generatedIdlPath, JSON.stringify(idl, null, 2));
}

async function generateTypeScriptSDK() {
  console.error("Generating TypeScript SDK to %s", generatedSDKDir);
  const generatedIdlPath = path.join(generatedIdlDir, `${PROGRAM_NAME}.json`);

  const idl = require(generatedIdlPath);
  const gen = new Solita(idl, { formatCode: true });
  await gen.renderAndWriteTo(generatedSDKDir);

  console.error("Success!");
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
