/**
 * Password scoring algorithm matching Verus-Mobile
 * Based on: https://github.com/Meyse/Verus-Mobile/blob/newsend3/src/utils/auth/scorePassword.js
 * 
 * Scores password 0-100 based on unique characters and character variation
 * Last Updated: Created to match Verus-Mobile password strength requirements
 */

const MIN_PASS_LENGTH = 7;
const MIN_PASS_SCORE = 65;
const PASS_SCORE_LIMIT = 100;

interface Variations {
  digits: RegExp;
  lower: RegExp;
  upper: RegExp;
  nonWords: RegExp;
}

function limitValue(value: number, minValue: number, maxValue: number): number {
  let currentValue = 0;
  if (!Number.isNaN(value)) {
    currentValue = parseInt(value.toString());
  }
  return Math.min(Math.max(currentValue, minValue), maxValue);
}

export function scorePassword(pass: string, minLength = MIN_PASS_LENGTH, limit = PASS_SCORE_LIMIT): number {
  const variations: Variations = {
    digits: /\d/,
    lower: /[a-z]/,
    upper: /[A-Z]/,
    nonWords: /\W/,
  };

  let score = 0;
  let variationCount = 0;
  const letters: Record<string, number> = {};

  if (!pass || pass.length < minLength) {
    return score;
  }

  /* Score unique letters until 5 repetitions */
  for (let i = 0; i < pass.length; i += 1) {
    letters[pass[i]] = (letters[pass[i]] || 0) + 1;
    score += 5.0 / letters[pass[i]];
  }

  /* Score character variation */
  Object.keys(variations).forEach((variation) => {
    const key = variation as keyof Variations;
    const variationCheck = variations[key].test(pass);
    variationCount += variationCheck === true ? 1 : 0;
  });
  score += (variationCount - 1) * 10;

  return limitValue(score, 0, limit);
}

export function getPasswordStrength(score: number): {
  level: number;
  color: string;
  label: string;
} {
  if (score === 0) {
    return { level: 0, color: 'muted', label: 'strength' };
  }
  
  if (score < MIN_PASS_SCORE) {
    return { level: 2, color: 'destructive', label: 'weak' };
  }
  
  // Score >= MIN_PASS_SCORE (65), distribute remaining levels
  const span = PASS_SCORE_LIMIT - MIN_PASS_SCORE;
  const adjusted = Math.max(0, Math.min(1, (score - MIN_PASS_SCORE) / span));
  const level = Math.min(5, 3 + Math.ceil(adjusted * 2)); // 3-5 bars
  
  if (level === 3) {
    return { level, color: 'blue-500', label: 'mediocre' };
  }
  if (level === 4) {
    return { level, color: 'primary', label: 'good' };
  }
  return { level: 5, color: 'green-500', label: 'excellent' };
}

export { MIN_PASS_LENGTH, MIN_PASS_SCORE, PASS_SCORE_LIMIT };
