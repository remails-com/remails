import {ReactNode} from 'react';
import {Dashboard} from './layout/Dashboard';
import {MessageLog} from './components/MessageLog';
import {OrganizationsOverview} from './components/organizations/OrganizationsOverview';
import {ProjectsOverview} from "./components/projects/ProjectsOverview.tsx";
import {StreamsOverview} from "./components/streams/StreamsOverview.tsx";
import {Project} from "./components/projects/Project.tsx";
import {useRemails} from "./hooks/useRemails.ts";
import {Stream} from "./components/streams/Stream.tsx";

export function Pages() {
  const {state: {route}} = useRemails();

  let element: ReactNode = route.name;

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

  if (route.name === 'stream') {
    element = <Stream />
  }

  return (
    <Dashboard>
      {element}
    </Dashboard>
  );
}
