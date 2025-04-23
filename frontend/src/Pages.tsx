import {ReactNode} from 'react';
import {useRouter} from './hooks/useRouter.ts';
import {Dashboard} from './layout/Dashboard';
import {MessageLog} from './components/MessageLog';
import {OrganizationsOverview} from './components/organizations/OrganizationsOverview';
import {ProjectsOverview} from "./components/projects/ProjectsOverview.tsx";
import {StreamsOverview} from "./components/streams/StreamsOverview.tsx";
import {Project} from "./components/projects/Project.tsx";

export function Pages() {
  const {route} = useRouter();

  let element: ReactNode = route.name;

  console.log('route.name', route.name)

  if (route.name === 'message-log') {
    element = <MessageLog/>
  }

  if (route.name === 'organizations') {
    element = <OrganizationsOverview/>
  }

  if (route.name === 'projects') {
    element = <ProjectsOverview/>
  }

  if (route.name === 'streams') {
    element = <StreamsOverview/>
  }

  if (route.name === 'project') {
    element = <Project/>
  }

  return (
    <Dashboard>
      {element}
    </Dashboard>
  );
}
