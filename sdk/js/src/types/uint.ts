export class Uint {
  public number: number;

  public constructor(number: number) {
    if (!Number.isInteger(number)) {
      throw new Error(`uint is not an integer: ${number}`);
    }
    if (number < 0) {
      throw new Error(`uint is less than zero: ${number}`);
    }
    this.number = number;
  }

  public static fromString(str: string): Uint {
    return new Uint(parseInt(str));
  }

  public toString(): string {
    return this.number.toString();
  }

  /**
   * Implementation for `JSON.parse`.
   */
  static parse(json: string): Uint {
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
  toJSON(): string {
    return this.toString();
  }
}
