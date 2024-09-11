import { Config } from '../types/binding.js';
import { ColorFunction, PersistentCacheBrand, colors } from './color.js';
import { ResolvedServerUrls } from './http.js';
import { clearScreen, formatExecutionTime, pad, version } from './share.js';

type LogLevelNames = 'trace' | 'debug' | 'info' | 'warn' | 'error';

enum LogLevel {
  Trace = 'trace',
  Debug = 'debug',
  Info = 'info',
  Warn = 'warn',
  Error = 'error'
}

export interface ILogger {
  trace(message: string): void;
  debug(message: string): void;
  info(message: string): void;
  warn(message: string): void;
  warnOnce(message: string): void;
  errorOnce(message: string | Error): void;
  error(message: string | Error, options?: ErrorOptions): void;
}

export interface ErrorOptions {
  exit?: boolean;
  e?: Error;
  error?: Error;
}
interface LoggerOptions {
  prefix?: string;
  customLogger?: Logger;
  allowClearScreen?: boolean;
  brandColor?: ColorFunction;
  exit?: boolean;
}

const LOGGER_METHOD = {
  info: 'log',
  warn: 'warn',
  error: 'error'
} as const;

const warnOnceMessages = new Set();
const infoOnceMessages = new Set();
const errorOnceMessages = new Set();

export class Logger implements ILogger {
  private clear: () => void = () => {};
  canClearScreen: boolean;
  colorMap: {
    trace: (input: string) => string;
    debug: (input: string) => string;
    info: (input: string) => string;
    warn: (input: string) => string;
    error: (input: string) => string;
  };
  constructor(
    public options?: LoggerOptions,
    private levelValues: Record<LogLevelNames, number> = {
      trace: 0,
      debug: 1,
      info: 2,
      warn: 3,
      error: 4
    }
  ) {
    if (!this.options) this.options = {};
    this.canClearScreen =
      this.options.allowClearScreen && process.stdout.isTTY && !process.env.CI;
    this.clear = this.canClearScreen ? clearScreen : () => {};
    this.colorMap = {
      trace: colors.green,
      debug: colors.debugColor,
      info: colors.brandColor,
      warn: colors.yellow,
      error: colors.red
    };
    this.brandPrefix();
  }

  private brandPrefix(color?: (s: string | string[]) => string): void {
    const { prefix = 'Farm' } = this.options;
    const formattedName = colors.bold(prefix);
    const formattedPrefix = colors.bold(`[ ${formattedName} ]`);
    this.options.prefix = color ? color(formattedPrefix) : formattedPrefix;
  }

  private logMessage(
    level: LogLevelNames,
    message: string | Error,
    color?: (s: string | string[]) => string,
    showBanner = true
  ): void {
    const loggerMethod =
      level in LOGGER_METHOD
        ? LOGGER_METHOD[level as keyof typeof LOGGER_METHOD]
        : 'log';
    if (this.levelValues[level] <= this.levelValues[level]) {
      this.canClearScreen && this.clear();
      const prefix = showBanner ? `${this.options.prefix} ` : '';
      const prefixColor = this.colorMap[level];
      const loggerMessage = color ? color(message as string) : message;
      console[loggerMethod](prefixColor(prefix) + loggerMessage);
    }
  }

  setPrefix(options: LoggerOptions): void {
    if (options.prefix) {
      this.options.prefix = options.prefix;
      this.brandPrefix(options.brandColor);
    }
  }

  trace(message: string): void {
    this.logMessage(LogLevel.Trace, message, colors.magenta);
  }

  debug(message: string): void {
    this.logMessage(LogLevel.Debug, message, colors.blue);
  }

  info(message: string): void {
    this.logMessage(LogLevel.Info, message, null);
  }

  warn(message: string): void {
    this.logMessage(LogLevel.Warn, message, colors.yellow);
  }

  error(message: string | Error, errorOptions?: ErrorOptions): void {
    const effectiveOptions = { ...this.options, ...errorOptions };
    const causeError = errorOptions?.e || errorOptions?.error;

    let error;

    if (typeof message === 'string') {
      error = new Error(message);
      error.stack = '';
    } else {
      error = message;
    }

    if (causeError) {
      error.message += `\nCaused by: ${causeError.stack ?? causeError}`;
    }

    this.logMessage(LogLevel.Error, error, colors.red);

    if (effectiveOptions.exit) {
      process.exit(1);
    }
  }
  infoOnce(message: string) {
    if (!infoOnceMessages.has(message)) {
      infoOnceMessages.add(message);
      this.info(message);
    }
  }
  warnOnce(message: string) {
    if (!warnOnceMessages.has(message)) {
      warnOnceMessages.add(message);
      this.warn(message);
    }
  }
  errorOnce(message: string | Error) {
    if (!errorOnceMessages.has(message)) {
      errorOnceMessages.add(message);
      this.error(message);
    }
  }
  hasErrorLogged(message: string | Error) {
    return errorOnceMessages.has(message);
  }
  hasWarnLogged(message: string) {
    return warnOnceMessages.has(message);
  }
}

// use in test
// TODO: impl ILogger
export class NoopLogger extends Logger {
  setPrefix(_options: LoggerOptions): void {}
  trace(_message: string): void {}
  debug(_message: string): void {}
  info(_message: string, _iOptions?: LoggerOptions): void {}
  warn(_message: string): void {}
  error(_message: string | Error, _errorOptions?: ErrorOptions): void {
    if (_errorOptions.exit) {
      let e = _message instanceof Error ? _message : new Error(_message);
      if (_errorOptions?.e || _errorOptions?.error) {
        e.cause = _errorOptions.e || _errorOptions.error;
      }

      throw e;
    }
  }
  infoOnce(_message: string): void {}
  warnOnce(_message: string): void {}
  errorOnce(_message: string | Error): void {}
  hasErrorLogged(_message: string | Error): boolean {
    return false;
  }
  hasWarnLogged(_message: string): boolean {
    return false;
  }
}

export function bootstrapLogger(options?: LoggerOptions): Logger {
  return new Logger(options);
}

export function bootstrap(times: number, config: Config, hasCacheDir: boolean) {
  const usePersistentCache = config.config.persistentCache && hasCacheDir;
  const persistentCacheFlag = usePersistentCache
    ? colors.bold(PersistentCacheBrand)
    : '';

  console.log(
    '\n',
    colors.bold(colors.brandColor(`${'ϟ'}  Farm  v${version}`))
  );

  console.log(
    `${colors.bold(colors.green(` ✓`))}  ${colors.bold(
      'Compile in'
    )} ${colors.bold(
      colors.green(formatExecutionTime(times, 's'))
    )} ${persistentCacheFlag}`,
    '\n'
  );
}

export const logger = new Logger();

export function buildErrorMessage(
  err: any,
  args: string[] = [],
  includeStack = true
): string {
  if (err.plugin) args.push(`  Plugin: ${colors.magenta(err.plugin)}`);
  const loc = err.loc ? `:${err.loc.line}:${err.loc.column}` : '';
  if (err.id) args.push(`  File: ${colors.cyan(err.id)}${loc}`);
  if (err.frame) args.push(colors.yellow(pad(err.frame)));
  if (includeStack && err.stack) args.push(pad(cleanStack(err.stack)));
  return args.join('\n');
}

function cleanStack(stack: string) {
  return stack
    .split(/\n/g)
    .filter((l) => /^\s*at/.test(l))
    .join('\n');
}

export function printServerUrls(
  urls: ResolvedServerUrls,
  optionsHost: string | boolean | undefined,
  logger: ILogger
): void {
  const colorUrl = (url: string) =>
    colors.cyan(url.replace(/:(\d+)\//, (_, port) => `:${colors.bold(port)}/`));
  for (const url of urls.local) {
    logger.info(
      `${colors.bold(colors.green('➜'))} ${colors.bold(
        'Local'
      )}:   ${colors.bold(colorUrl(url))}`
    );
  }
  for (const url of urls.network) {
    logger.info(
      `${colors.bold(colors.green('➜'))} ${colors.bold(
        'Network'
      )}: ${colors.bold(colorUrl(url))}`
    );
  }
  if (urls.network.length === 0 && optionsHost === undefined) {
    logger.info(
      colors.dim(`  ${colors.green('➜')}  ${colors.bold('Network')}: use `) +
        colors.bold('--host') +
        colors.dim(' to expose')
    );
  }
}
