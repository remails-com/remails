import { ReactNode } from 'react';
import { useRouter } from './hooks/useRouter';
import { Dashboard } from './layout/Dashboard'; 
import { MessageLog } from './MessageLog';

export function Pages() {
  const { route } = useRouter();

  let element: ReactNode = route.name;

  if (route.name === 'message-log') {
    element = <MessageLog />
  }

  return (
    <Dashboard>
      {element}
    </Dashboard>
  );
}
