import {createContext, useContext, useEffect, useState} from 'react';
import {BreadcrumbItem} from "../types.ts";

export type RouteName = string;
export type RouteParams = Record<string, string>;
export type Navigate = (name: RouteName, params?: RouteParams) => void;

export interface Route {
  name: RouteName;
  path: string;
  display: string;
  children?: Route[];
}

export interface RouterContextProps {
  route: Route;
  fullPath: string;
  fullName: string;
  params: RouteParams;
  breadcrumbItems: BreadcrumbItem[];
  navigate: Navigate;
}

export const routes: Route[] = [
  {
    name: 'projects',
    display: 'Projects',
    path: '/projects',
    children: [
      {
        name: 'project',
        display: '{currentProject.name}',
        path: '/{proj_id}',
        children: [
          {
            name: 'domains',
            display: 'Domains',
            path: '/domains',
          },
          {
            name: 'streams',
            display: 'Streams',
            path: '/streams',
            children: [
              {
                name: 'stream',
                display: '{currentStream.name}',
                path: '/{stream_id}',
                children: [
                  {
                    name: 'credentials',
                    display: 'Credentials',
                    path: '/credentials',
                  },
                  {
                    name: 'message-log',
                    display: 'Messages',
                    path: '/messages',
                  },
                ]
              },
            ]
          }
        ]
      },
    ]
  },
  {
    name: 'domains',
    display: 'Domains',
    path: '/domains',
  },
  {
    name: 'organizations',
    display: 'Organizations',
    path: '/organizations',
    children: [
      {
        name: 'add',
        display: 'Add Organization',
        path: '/add',
      },
      {
        name: 'edit',
        display: 'Edit Organization',
        path: '/edit/{id}',
      }
    ]
  },
];

export function matchPath(path: string): {
  route: Route,
  params: RouteParams,
  fullName: string,
  breadcrumbItems: BreadcrumbItem[]
} | null {
  const match = matchPathRecursive(path, routes, {}, [], []);
  if (match) {
    return {...match, fullName: match.fullName.join('.')}
  } else {
    return null
  }
}

// TODO remove logging

function matchPathRecursive(
  path: string,
  routes: Route[],
  pathParams: {
    [k: string]: string
  },
  fullName: string[],
  breadcrumbItems: BreadcrumbItem[]):
  {
    route: Route,
    params: RouteParams,
    fullName: string[],
    breadcrumbItems: BreadcrumbItem[]
  } | null {
  const new_path_params: { [k: string]: string } = {};
  const new_breadcrumb_items = [...breadcrumbItems]
  path = path.replace(/^\/|\/$/, '');
  const path_elems = path.split('/');
  route_loop:
    for (const route of routes) {
      const route_path = route.path.replace(/^\/|\/$/, '')
      const route_elems = route_path.split('/');
      for (const [index, route_elem] of route_elems.entries()) {
        if (route_elem !== path_elems[index]) {
          const path_var = route_elem.match(/^{(\w*)}$/)?.at(1);
          if (path_var) {
            new_path_params[path_var] = path_elems[index];
          } else {
            continue route_loop;
          }
        }
      }
      const new_full_name = [...fullName, route.name]
      new_breadcrumb_items.push({title: route.display, route: new_full_name.join('.')})
      if (route.children && path_elems.length > route_elems.length) {
        const rec_res = matchPathRecursive(path_elems.slice(route_elems.length).join('/'), route.children, new_path_params, new_full_name, new_breadcrumb_items);
        if (rec_res) {
          return {
            route: rec_res.route,
            params: {...pathParams, ...rec_res.params, ...new_path_params},
            fullName: rec_res.fullName,
            breadcrumbItems: rec_res.breadcrumbItems
          }
        } else {
          return null;
        }
      } else {
        return {
          route,
          params: {...pathParams, ...new_path_params},
          fullName: new_full_name,
          breadcrumbItems: new_breadcrumb_items
        };
      }
    }
  return null;
}

export function matchName(name: RouteName): {
  route: Route,
  fullPath: string,
  fullName: string,
  breadcrumbItems: BreadcrumbItem[]
} {
  const elems = name.split('.');
  const match = recursiveMatchName(routes, elems, '', [], [])
  if (match) {
    return {...match, fullName: match.fullName.join('.')}
  }
  return {
    route: routes[0],
    fullPath: routes[0].path,
    fullName: routes[0].name,
    breadcrumbItems: [{route: routes[0].name, title: routes[0].display}]
  };
}

function recursiveMatchName(routes: Route[], name_elems: string[], fullPath: string, fullName: string[], breadcrumbItems: BreadcrumbItem[]): {
  route: Route,
  fullPath: string,
  fullName: string[]
  breadcrumbItems: BreadcrumbItem[]
} | null {
  const elem = name_elems.at(0);
  if (!elem) {
    return null
  }
  for (const route of routes) {
    if (route.name === elem) {
      const newFullName = [...fullName, route.name]
      breadcrumbItems.push({title: route.display, route: newFullName.join('.')});
      if (name_elems.length > 1 && route.children) {
        return recursiveMatchName(route.children, name_elems.slice(1), `${fullPath}/${route.path.replace(/^\/|\/$/, '')}`, newFullName, breadcrumbItems)
      }
      return {
        route,
        fullPath: `${fullPath}/${route.path.replace(/^\/|\/$/, '')}`,
        fullName: newFullName,
        breadcrumbItems
      };
    }
  }
  return null
}

export const RouterContext = createContext<RouterContextProps>({
  route: routes[0],
  fullPath: routes[0].path,
  fullName: routes[0].name,
  params: {},
  breadcrumbItems: [],
  navigate: () => {
  },
});

export function useRouter(): RouterContextProps {
  return useContext(RouterContext);
}

export function useInitRouter(): RouterContextProps {
  const currentPath = window.location.pathname;
  const {
    route: init_route,
    params: path_params,
    fullName: initFullName,
    breadcrumbItems: path_breadcrumbItems
  } = matchPath(currentPath) || {
    route: routes[0],
    params: {},
    fullName: routes[0].name,
    breadcrumbItems: [{title: routes[0].display, route: routes[0].name}]
  };
  const [route, setRoute] = useState<{
    route: Route,
    fullPath: string,
    fullName: string,
    breadcrumbItems: BreadcrumbItem[]
  }>({route: init_route, fullPath: currentPath, fullName: initFullName, breadcrumbItems: path_breadcrumbItems});

  const queryString = new URLSearchParams(window.location.search);
  const [params, setParams] = useState<RouteParams>(
    {...Object.fromEntries(queryString), ...path_params}
  );

  // handle back / forward events
  useEffect(() => {
    window.addEventListener('popstate', (event) => {
      if (event.state?.routeName) {
        setRoute(matchName(event.state.routeName));
      } else {
        setRoute({
          route: init_route,
          fullPath: currentPath,
          fullName: initFullName,
          breadcrumbItems: path_breadcrumbItems
        });
        setParams({...Object.fromEntries(queryString), ...path_params});
      }
      if (event.state?.routeParams) {
        setParams(event.state.routeParams);
      }
    });
  }, []);

  // navigate to a new route
  const navigate = (name: RouteName, params: RouteParams = {}) => {
    let {route: newRoute, fullPath: path, breadcrumbItems} = matchName(name);

    if (params && Object.keys(params).length > 0) {
      const queryParams: { [k: string]: string } = {};
      for (const key of Object.keys(params)) {
        if (path.includes(`{${key}}`)) {
          path = path.replace(`{${key}}`, params[key])
        } else {
          queryParams[key] = params[key];
        }
      }
      const searchParams = new URLSearchParams(queryParams);
      const fullPath = Object.keys(queryParams).length === 0 ? path : `${path}?${searchParams}`;
      window.history.pushState(
        {routeName: name, routeParams: params},
        '',
        fullPath
      );
      setRoute({route: newRoute, fullPath: path, fullName: initFullName, breadcrumbItems});
      setParams(params);
    } else {
      window.history.pushState(
        {routeName: name, routeParams: {}},
        '',
        path
      );
      setRoute({route: newRoute, fullPath: path, fullName: initFullName, breadcrumbItems});
      setParams({});
    }
  };

  return {
    params,
    route: route.route,
    fullPath: route.fullPath,
    fullName: route.fullName,
    breadcrumbItems: route.breadcrumbItems,
    navigate,
  };
}