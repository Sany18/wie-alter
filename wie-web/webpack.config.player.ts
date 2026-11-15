import path from "path";

import webpack from "webpack";
import { merge } from "webpack-merge";
import CopyPlugin from "copy-webpack-plugin";
// @ts-ignore: allowImportingTsExtensions
import commonConfig from "./webpack.config.common.ts";

// Embeddable player (src/ts/player.ts): one script exposing `window.WiePlayer`, the wasm module, font and MIDI
// assets, all loaded relative to the script so the bundle can be served from any path.
const base = merge(commonConfig, {
  mode: "production",
  devtool: false,
  entry: { player: "./src/ts/player.ts" },
  output: {
    path: path.resolve(import.meta.dirname, "dist-player"),
    filename: "wie-player.js",
    // One script: the shell lists its files by name (sw.config.json), so no numbered chunks.
    asyncChunks: false,
    // The one remaining chunk is the audio Web Worker: a fixed name for the same reason.
    chunkFilename: "wie-[name].js",
    webassemblyModuleFilename: "wie.wasm",
    publicPath: "auto",
    library: { name: "WiePlayer", type: "self" },
    clean: true,
  },
  module: {
    rules: [{ test: /\.ttf$/, type: "asset/resource", generator: { filename: "[name][ext]" } }],
  },
});

// Only the player's runtime files: none of the site's public folder.
const config: webpack.Configuration = {
  ...base,
  entry: { player: "./src/ts/player.ts" },
  plugins: [
    ...(base.plugins ?? []).filter(plugin => !(plugin instanceof CopyPlugin)),
    // The wasm glue would otherwise become a numbered chunk.
    new webpack.optimize.LimitChunkCountPlugin({ maxChunks: 1 }),
    new CopyPlugin({
      patterns: [
        { from: path.resolve(import.meta.dirname, "../assets/1mgm.sf2"), to: "." },
        { from: path.resolve(import.meta.dirname, "../assets/1mgm.txt"), to: "." },
        { from: path.resolve(import.meta.dirname, "../assets/neodgm.txt"), to: ".", noErrorOnMissing: true },
        {
          from: path.resolve(import.meta.dirname, "../node_modules/spessasynth_lib/dist/spessasynth_processor.min.js"),
          to: ".",
        },
      ],
    }),
  ],
};

export default config;
