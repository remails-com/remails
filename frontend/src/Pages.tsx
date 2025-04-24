import {ReactNode} from 'react';
import {Dashboard} from './layout/Dashboard';
import {OrganizationsOverview} from './components/organizations/OrganizationsOverview';
import {ProjectsOverview} from "./components/projects/ProjectsOverview.tsx";
import {useRemails} from "./hooks/useRemails.ts";
import {StreamDetails} from "./components/streams/StreamDetails.tsx";
import {ProjectDetails} from "./components/projects/ProjectDetails.tsx";

export function Pages() {
  const {state: {route}} = useRemails();

  let element: ReactNode;

  switch (route.name) {
    case 'organizations':
      element = <OrganizationsOverview/>
      break
    case 'projects':
      element = <ProjectsOverview/>
      break
    case 'project':
      element = <ProjectDetails/>
      break
    case 'stream':
      element = <StreamDetails/>
      break
    default:
      element = "Not Found"
  }

  return (
    <Dashboard>
      {element}
    </Dashboard>
  );
}
