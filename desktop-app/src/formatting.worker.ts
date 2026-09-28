import { formatLatex } from "./latexFormatting";
import type { FormatRequest } from "./formattingTypes";

self.onmessage = async (event: MessageEvent<FormatRequest>) => {
  try {
    self.postMessage({
      text: await formatLatex(event.data.source, event.data.mode),
    });
  } catch (error) {
    self.postMessage({
      error: error instanceof Error ? error.message : String(error),
    });
  }
};
