import webpack from "webpack";
import { merge } from "webpack-merge";
// @ts-ignore: allowImportingTsExtensions
import commonConfig from "./webpack.config.common.ts";

// Production build without Sentry upload (local demo).
const config: webpack.Configuration = merge(commonConfig, {
  mode: "production",
  devtool: false,
});
export default config;
