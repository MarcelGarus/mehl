import * as child_process from "child_process";
import * as stream from "stream";
import * as vs from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  StreamInfo,
} from "vscode-languageclient/node";
import { TypeLabelsDecorations } from "./type_labels";

let client: LanguageClient;

export async function activate(context: vs.ExtensionContext) {
  console.log("Activated Mehl extension!");

  let clientOptions: LanguageClientOptions = {
    outputChannelName: "Mehl Analysis Server",
  };

  client = new LanguageClient(
    "mehlAnalysisLSP",
    "Mehl Analysis Server",
    spawnServer,
    clientOptions
  );
  client.start();

  context.subscriptions.push(new TypeLabelsDecorations(client));
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}

// The following code is taken (and slightly modified) from https://github.com/Dart-Code/Dart-Code
async function spawnServer(): Promise<StreamInfo> {
  const process = safeSpawn();
  console.info(`    PID: ${process.pid}`);

  const reader = process.stdout.pipe(new LoggingTransform("<=="));
  const writer = new LoggingTransform("==>");
  writer.pipe(process.stdin);

  process.stderr.on("data", (data) => console.error(data.toString()));

  return { reader, writer };
}

type SpawnedProcess = child_process.ChildProcess & {
  stdin: stream.Writable;
  stdout: stream.Readable;
  stderr: stream.Readable;
};
function safeSpawn(): SpawnedProcess {
  const configuration = vs.workspace.getConfiguration("mehl");

  let command: [string, string[]] = ["mehl.exe", ["lsp"]];
  const languageServerCommand = configuration.get<string>(
    "languageServerCommand"
  );
  if (
    languageServerCommand != null &&
    languageServerCommand.trim().length !== 0
  ) {
    const parts = languageServerCommand.split(" ");
    command = [parts[0], parts.slice(1)];
  }

  const corePath = configuration.get<string | null>("corePath");
  command[1].push(`--core-path="${corePath}"`);

  return child_process.spawn(command[0], command[1], {
    env: process.env,
    shell: true,
  }) as SpawnedProcess;
}
class LoggingTransform extends stream.Transform {
  constructor(
    private readonly prefix: string,
    private readonly onlyShowJson: boolean = true,
    opts?: stream.TransformOptions
  ) {
    super(opts);
  }
  public _transform(
    chunk: any,
    encoding: BufferEncoding,
    callback: () => void
  ): void {
    let value = (chunk as Buffer).toString();
    if (value.startsWith("Observatory listening on")) {
      console.warn(value);
      callback();
      return;
    }

    let toLog = this.onlyShowJson
      ? value
          .split("\r\n")
          .filter(
            (line) => line.trim().startsWith("{") || line.trim().startsWith("#")
          )
          .join("\r\n")
      : value;
    if (toLog.length > 0 || !this.onlyShowJson) {
      console.info(`${this.prefix} ${toLog}`);
    }

    this.push(chunk, encoding);
    callback();
  }
}
