export class RemailsError extends Error {
  status: number;
  
  constructor(message: string, status: number) {
    super(message);
    this.status = status;
  }
}