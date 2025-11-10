import Big from "big.js";

export type BigSource = string | number | Big;

export type DecimalConstructor = {
  (value: BigSource): Decimal;
  new (value: BigSource): Decimal;
  readonly ZERO: Decimal;
  DP: number;
  RM: number;
  from(value: BigSource): Decimal;
  max(...values: (string | number | Decimal)[]): Decimal;
};

Big.DP = 18;

class Decimal {
  readonly inner: Big;

  constructor(value: BigSource) {
    try {
      this.inner = new Big(value);
    } catch (error) {
      throw new Error(`Invalid input for Decimal: "${value}". ${error}`);
    }
  }

  static max(...values: (string | number | Decimal)[]): Decimal {
    if (values.length === 0) {
      throw new Error("Decimal.max requires at least one argument");
    }
    return values.reduce((max, current) => {
      const currentDecimal = Decimal.from(current);
      return currentDecimal.gt(max) ? currentDecimal : max;
    }, Decimal.from(values[0])) as Decimal;
  }

  static from(value: string | number | Decimal): Decimal {
    if (value instanceof Decimal) return value;
    return new Decimal(value);
  }

  round(dp: number, rm: number): Decimal {
    return new Decimal(this.inner.round(dp, rm as Big.RoundingMode));
  }

  plus(num: string | number | Decimal): Decimal {
    const other = Decimal.from(num);
    const result = this.inner.plus(other.inner);
    return new Decimal(result);
  }

  minus(num: string | number | Decimal): Decimal {
    const other = Decimal.from(num);
    const result = this.inner.minus(other.inner);
    return new Decimal(result);
  }

  mul(num: string | number | Decimal): Decimal {
    const other = Decimal.from(num);
    const result = this.inner.mul(other.inner);
    return new Decimal(result);
  }

  mulCeil(num: string | number | Decimal): Decimal {
    const previousRm = Big.RM;
    Big.RM = Big.roundUp;
    const result = this.mul(num);
    Big.RM = previousRm;
    return result;
  }

  times(num: string | number | Decimal): Decimal {
    const other = Decimal.from(num);
    const result = this.inner.times(other.inner);
    return new Decimal(result);
  }

  div(num: string | number | Decimal): Decimal {
    const other = Decimal.from(num);
    if (other.isZero()) return new Decimal(0);
    const result = this.inner.div(other.inner);
    return new Decimal(result);
  }

  divCeil(num: string | number | Decimal): Decimal {
    const previousRm = Big.RM;
    Big.RM = Big.roundUp;
    const result = this.div(num);
    Big.RM = previousRm;
    return result;
  }

  divFloor(num: string | number | Decimal): Decimal {
    const previousRm = Big.RM;
    Big.RM = Big.roundDown;
    const result = this.div(num);
    Big.RM = previousRm;
    return result;
  }

  eq(num: string | number | Decimal): boolean {
    const other = Decimal.from(num);
    return this.inner.eq(other.inner);
  }

  gt(num: string | number | Decimal): boolean {
    const other = Decimal.from(num);
    return this.inner.gt(other.inner);
  }

  gte(num: string | number | Decimal): boolean {
    const other = Decimal.from(num);
    return this.inner.gte(other.inner);
  }

  lt(num: string | number | Decimal): boolean {
    const other = Decimal.from(num);
    return this.inner.lt(other.inner);
  }

  lte(num: string | number | Decimal): boolean {
    const other = Decimal.from(num);
    return this.inner.lte(other.inner);
  }

  isZero(): boolean {
    return this.inner.eq(0);
  }

  abs(): Decimal {
    return new Decimal(this.inner.abs());
  }

  neg(): Decimal {
    return new Decimal(this.inner.neg());
  }

  pow(exponent: number): Decimal {
    if (!Number.isInteger(exponent)) {
      throw new Error(`Exponent must be an integer, received: ${exponent}`);
    }
    return new Decimal(this.inner.pow(exponent));
  }

  toString(): string {
    return this.inner.toString();
  }

  toFixed(decimalPlaces?: number, rm?: number): string {
    return this.inner.toFixed(decimalPlaces, rm as Big.RoundingMode);
  }

  toNumber(): number {
    return this.inner.toNumber();
  }
}

const DecimalFactory = ((value: BigSource) => new Decimal(value)) as DecimalConstructor;

Object.defineProperty(DecimalFactory, "DP", {
  get(): number {
    return Big.DP;
  },
  set(value: number) {
    Big.DP = value;
  },
});

Object.defineProperty(DecimalFactory, "RM", {
  get(): number {
    return Big.RM;
  },
  set(value: number) {
    Big.RM = value;
  },
});

Object.defineProperty(DecimalFactory, "ZERO", {
  value: new Decimal("0"),
  writable: false,
});

Object.defineProperty(DecimalFactory, "from", {
  value: Decimal.from,
  writable: false,
});

Object.defineProperty(DecimalFactory, "max", {
  value: Decimal.max,
  writable: false,
});

export default DecimalFactory;
