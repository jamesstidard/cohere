export interface ValidationError {
  message: string;
  path?: string;
  value?: string;
  line?: number;
  column?: number;
}

export interface ValidationResult {
  valid: boolean;
  errors: ValidationError[];
}

export class Schema {
  constructor(schema: string | object);
  validateJson(data: string): ValidationResult;
  validateToml(data: string): ValidationResult;
}

export default function init(): Promise<void>;
