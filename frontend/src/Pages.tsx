import {ReactNode} from 'react';
import {Dashboard} from './layout/Dashboard';
import {OrganizationsOverview} from './components/organizations/OrganizationsOverview';
import {ProjectsOverview} from "./components/projects/ProjectsOverview.tsx";
import {useRemails} from "./hooks/useRemails.ts";
import {StreamDetails} from "./components/streams/StreamDetails.tsx";

export function Pages() {
  const {state: {route, fullName}} = useRemails();

  let element: ReactNode = route.name;

  if (route.name === 'organizations') {
    element = <OrganizationsOverview/>
  }

  if (fullName.startsWith('projects')) {
    element = <ProjectsOverview/>
  }

  if (route.name === 'stream') {
    element = <StreamDetails />
  }

  return (
    <Dashboard>
      {element}
    </Dashboard>
  );
}
