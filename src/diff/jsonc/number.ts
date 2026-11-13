const MAX_EXPANDED_DIGITS = 1000;
const NUMBER_PATTERN =
  /-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/y;

export interface NumberToken {
  readonly raw: string;
  readonly end: number;
}

function isDelimiter(text: string, index: number): boolean {
  if (index >= text.length) {
    return true;
  }
  const char = text[index] ?? "";
  return (
    char === " " ||
    char === "\t" ||
    char === "\n" ||
    char === "\r" ||
    char === "," ||
    char === "}" ||
    char === "]" ||
    char === "/"
  );
}

export function matchNumber(
  text: string,
  index: number,
): NumberToken | null {
  NUMBER_PATTERN.lastIndex = index;
  const match = NUMBER_PATTERN.exec(text);
  if (match === null) {
    return null;
  }
  const raw = match[0];
  const end = index + raw.length;
  if (!isDelimiter(text, end)) {
    return null;
  }
  return { raw, end };
}

function scientific(digits: string, exponent: bigint): string {
  const mantissa =
    digits.length === 1 ? digits : `${digits[0] ?? ""}.${digits.slice(1)}`;
  const scientificExponent = exponent + BigInt(digits.length - 1);
  return `${mantissa}e${scientificExponent.toString()}`;
}

export function canonicalNumber(raw: string): string {
  let sign = "";
  let body = raw;
  if (body.startsWith("-")) {
    sign = "-";
    body = body.slice(1);
  }

  let exponentText = "0";
  const exponentAt = body.search(/[eE]/);
  if (exponentAt >= 0) {
    exponentText = body.slice(exponentAt + 1);
    body = body.slice(0, exponentAt);
  }

  const dotAt = body.indexOf(".");
  const fractionLength = dotAt >= 0 ? body.length - dotAt - 1 : 0;
  let digits =
    dotAt >= 0 ? body.slice(0, dotAt) + body.slice(dotAt + 1) : body;
  const exponentSource = exponentText.startsWith("+")
    ? exponentText.slice(1)
    : exponentText;
  let exponent =
    BigInt(exponentSource.length === 0 ? "0" : exponentSource) -
    BigInt(fractionLength);

  digits = digits.replace(/^0+/, "");
  if (digits.length === 0) {
    return "0";
  }

  let end = digits.length;
  while (end > 1 && digits.charCodeAt(end - 1) === 0x30) {
    end -= 1;
    exponent += 1n;
  }
  digits = digits.slice(0, end);

  if (exponent >= 0n) {
    if (exponent <= BigInt(MAX_EXPANDED_DIGITS)) {
      return `${sign}${digits}${"0".repeat(Number(exponent))}`;
    }
    return `${sign}${scientific(digits, exponent)}`;
  }

  const places = -exponent;
  if (places < BigInt(digits.length)) {
    const point = digits.length - Number(places);
    return `${sign}${digits.slice(0, point)}.${digits.slice(point)}`;
  }
  const zeros = places - BigInt(digits.length);
  if (zeros <= BigInt(MAX_EXPANDED_DIGITS)) {
    return `${sign}0.${"0".repeat(Number(zeros))}${digits}`;
  }
  return `${sign}${scientific(digits, exponent)}`;
}
