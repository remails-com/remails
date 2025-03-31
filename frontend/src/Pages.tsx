import { ReactNode } from 'react';
import { useRouter } from './hooks/useRouter';
import { Dashboard } from './layout/Dashboard';
import { MessageLog } from './components/MessageLog';
import { OrganizationsOverview } from './components/organizations/OrganizationsOverview';

export function Pages() {
  const { route } = useRouter();

  let element: ReactNode = route.name;

  if (route.name === 'message-log') {
    element = <MessageLog />
  }

  if (route.name === 'organizations') {
    element = <OrganizationsOverview />
  }

  return (
    <Dashboard>
      {element}
    </Dashboard>
  );
}
