/**
 * Represents a prepared SQL statement.
 */
export class Statement {
  /**
   * Finalizes the statement, releasing any resources associated with it.
   */
  finalize(): void;
}

export class Row {
  get(nameOrIndex: number | string): SqlValue | null;
  columnName(idx: number): string | null;
  length(): number;
  toJSON(): unknown;
}

export type SqlArrayValue = Array<SqlValue>;

export type SqlValue = string | number | Date | boolean | SqlArrayValue;

/**
 * Interface for executing SQL queries and commands.
 */
export interface Executor {
  /**
   * Executes a query and returns the resulting rows.
   *
   * @param stmt - The SQL statement to execute, either as a `Statement` object or a string.
   * @param params - Optional parameters to bind to the statement.
   * @returns A promise that resolves to an array of rows.
   */
  query(stmt: Statement | string, params?: SqlValue[]): Promise<Row[]>;

  /**
   * Executes a command without returning any rows.
   *
   * @param stmt - The SQL statement to execute, either as a `Statement` object or a string.
   * @param params - Optional parameters to bind to the statement.
   * @returns A promise that resolves when the command is complete.
   */
  exec(stmt: Statement | string, params?: SqlValue[]): Promise<void>;
}

/**
 * Represents a database connection that can execute SQL statements and manage transactions.
 */
interface Conn extends Executor {
  /**
   * Executes a function within a transaction. If the function throws an error,
   * the transaction is rolled back. Otherwise, it is committed.
   *
   * @param func - A function that receives an `Executor` to execute queries within the transaction.
   * @returns A promise that resolves to the result of the function.
   */
  transaction<R>(func: (conn: Executor) => Promise<R> | R): Promise<R>;
}

/**
 * Represents a pool of database connections.
 */
export class Pool {
  /**
   * Retrieves a connection from the pool.
   *
   * @returns A promise that resolves to a `Conn` object.
   */
  get(): Promise<Conn>;

  /**
   * Opens a new connection pool with the specified configuration.
   *
   * @param config - The configuration for the database connection.
   * @returns A promise that resolves to a `Pool` object.
   */
  static open(config: Config): Promise<Pool>;
}

/**
 * Configuration options for the database connection.
 */
export type Config =
  | { type: "sqlite"; path?: string }
  | { type: "libsql"; path?: string };
