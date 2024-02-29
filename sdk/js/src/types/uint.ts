export class Uint {
  public number: number;

  /**
   * Create a new `Uint` instance from a non-negative integer number.
   */
  public constructor(number: number) {
    if (!Number.isInteger(number)) {
      throw new Error(`uint is not an integer: ${number}`);
    }
    if (number < 0) {
      throw new Error(`uint is less than zero: ${number}`);
    }
    this.number = number;
  }

  /**
   * Create a new `Uint` instance from a string that represents a non-negative
   * integer number.
   */
  public static fromString(str: string): Uint {
    return new Uint(parseInt(str));
  }

  /**
   * Stringify the `Uint`.
   */
  public toString(): string {
    return this.number.toString();
  }

  /**
   * Implementation for `JSON.parse`.
   */
  public static fromJSON(json: string): Uint {
    return JSON.parse(json, (_, value) => {
      if (typeof value === "string") {
        return new Number(value);
      }
      return value;
    });
  }

  /**
   * Implementation for `JSON.stringify`.
   */
  public toJSON(): string {
    return this.toString();
  }
}
