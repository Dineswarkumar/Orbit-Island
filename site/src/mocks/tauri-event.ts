export interface MockEvent<T> {
  event: string;
  id: number;
  payload: T;
}

export type MockUnlisten = () => void;
export type MockEventHandler<T> = (event: MockEvent<T>) => void;

export async function listen<T>(
  _eventName: string,
  _handler: MockEventHandler<T>,
): Promise<MockUnlisten> {
  return () => undefined;
}

export async function emit<T>(_eventName: string, _payload?: T): Promise<void> {
  return undefined;
}

export async function invoke<T>(_cmd: string, _args?: unknown): Promise<T> {
  return undefined as unknown as T;
}
