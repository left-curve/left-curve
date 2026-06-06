import { describe, expect, it } from "vitest";

import imagePathTransformLoader from "../rsbuild.image-path-loader";

type TransformResult = {
  code: string;
  errors: Error[];
  warnings: Error[];
};

const transform = (source: string): TransformResult => {
  const errors: Error[] = [];
  const warnings: Error[] = [];
  let code = source;

  const returned = imagePathTransformLoader.call(
    {
      async() {
        return (error, transformed) => {
          if (error) throw error;
          code = transformed ?? "";
        };
      },
      cacheable() {},
      emitError(error) {
        errors.push(error);
      },
      emitWarning(warning) {
        warnings.push(warning);
      },
      resourcePath: "sample.tsx",
    },
    source,
    null,
  );

  if (typeof returned === "string") code = returned;

  return { code, errors, warnings };
};

describe("rsbuild image path loader", () => {
  it("rewrites JSX literal image src values", () => {
    const { code, warnings } = transform('const element = <img src="/images/coins/usd.svg" />;');

    expect(warnings).toHaveLength(0);
    expect(code).toContain('import __dangoImageAsset0 from "~/images/coins/usd.svg";');
    expect(code).toContain("src={__dangoImageAsset0}");
  });

  it("memoizes repeated literal imports", () => {
    const { code } = transform(
      'const first = "/images/coins/usd.svg"; const second = "/images/coins/usd.svg";',
    );

    expect(code.match(/import __dangoImageAsset/g)).toHaveLength(1);
  });

  it("preserves query and fragment suffixes in image requests", () => {
    const { code } = transform('const icon = "/images/icons/search.svg?raw#sprite";');

    expect(code).toContain('from "~/images/icons/search.svg?raw#sprite"');
  });

  it("rewrites constrained dynamic templates with a narrowed context", () => {
    const { code, warnings } = transform(
      'const logo = `/images/union${theme === "dark" ? "-dark" : ""}.png`;',
    );

    expect(warnings).toHaveLength(0);
    expect(code).toContain('import * as __dangoImageAssetSentry from "@sentry/react";');
    expect(code).toContain('require.context("~/images", false, /^\\.\\/union.*\\.png$/i)');
    expect(code).toContain(
      '__dangoImageAssetUrl0(`union${theme === "dark" ? "-dark" : ""}.png`)',
    );
  });

  it("marks dynamic contexts recursive when the generated key contains a slash", () => {
    const { code } = transform(
      "const frame = `/images/points/boxes-animation/${variant}/frame_${index}.webp`;",
    );

    expect(code).toContain(
      'require.context("~/images/points/boxes-animation", true, /^\\.\\/.*\\/frame_.*\\.webp$/i)',
    );
  });

  it("emits a Sentry-reported fallback for missing dynamic context keys", () => {
    const { code } = transform("const box = `/images/points/boxes/${tier}.png`;");

    expect(code).toContain("try {");
    expect(code).toContain('const fallbackPath = "/images/points/boxes/" + path;');
    expect(code).toContain("__dangoImageAssetSentry.captureException(error");
    expect(code).toContain("return fallbackPath;");
  });

  it("warns and leaves unsupported dynamic image templates unchanged", () => {
    const source = "const image = `/images/points/${name}`;";
    const { code, warnings } = transform(source);

    expect(code).toBe(source);
    expect(warnings).toHaveLength(1);
    expect(warnings[0].message).toContain("must end with a supported image extension");
  });

  it("warns instead of creating a broad root image context", () => {
    const source = "const image = `/images/${name}.png`;";
    const { code, warnings } = transform(source);

    expect(code).toBe(source);
    expect(warnings).toHaveLength(1);
    expect(warnings[0].message).toContain("static filename prefix");
  });

  it("emits dynamic template warnings as errors in CI", () => {
    const originalCi = process.env.CI;
    process.env.CI = "true";

    try {
      const { errors, warnings } = transform("const image = `/images/points/${name}`;");

      expect(warnings).toHaveLength(1);
      expect(errors).toHaveLength(1);
      expect(errors[0]).toBe(warnings[0]);
    } finally {
      if (originalCi === undefined) {
        delete process.env.CI;
      } else {
        process.env.CI = originalCi;
      }
    }
  });

  it("rewrites static template literals", () => {
    const { code } = transform("const icon = `/images/emojis/simple/map.svg`;");

    expect(code).toContain('import __dangoImageAsset0 from "~/images/emojis/simple/map.svg";');
    expect(code).toContain("const icon = __dangoImageAsset0;");
  });

  it("skips object keys, import sources, directives, and type-only literals", () => {
    const sources = [
      'const values = { "/images/coins/usd.svg": true };',
      'import icon from "/images/coins/usd.svg";',
      '"/images/coins/usd.svg"; const value = true;',
      'type Icon = "/images/coins/usd.svg";',
    ];

    for (const source of sources) {
      const { code, warnings } = transform(source);

      expect(code).toBe(source);
      expect(warnings).toHaveLength(0);
    }
  });

  it("ignores non-image strings and image-like paths inside comments", () => {
    const sources = ['const readme = "/images/readme.txt";', "// /images/coins/usd.svg"];

    for (const source of sources) {
      const { code, warnings } = transform(source);

      expect(code).toBe(source);
      expect(warnings).toHaveLength(0);
    }
  });

  it("uses the fast path when the source cannot contain image paths", () => {
    const source = "const value = 1;";
    const { code, warnings } = transform(source);

    expect(code).toBe(source);
    expect(warnings).toHaveLength(0);
  });
});
