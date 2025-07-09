import Big from "big.js";

export type BigSource = string | number | Big;

export type DecimalConstructor = {
  (value: BigSource): Decimal;
  new (value: BigSource): Decimal;
  readonly ZERO: Decimal;
  DP: number;
  RM: number;
  from(value: BigSource): Decimal;
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

  static from(value: string | number | Decimal): Decimal {
    if (value instanceof Decimal) return value;
    return new Decimal(value);
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

  toFixed(decimalPlaces?: number): string {
    return this.inner.toFixed(decimalPlaces);
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

export default DecimalFactory;
