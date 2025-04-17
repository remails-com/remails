import {createContext, useContext, useEffect, useState} from 'react';

export type RouteName = string;
export type RouteParams = Record<string, string>;
export type Navigate = (name: RouteName, params?: RouteParams) => void;

export interface Route {
  name: RouteName;
  path: string;
  children?: Route[];
}

export interface RouterContextProps {
  route: Route;
  fullPath: string;
  params: RouteParams;
  navigate: Navigate;
}

export const routes: Route[] = [
  {
    name: 'projects',
    path: '/projects',
    children: [
      {
        name: 'domains',
        path: '/{proj_id}/domains',
      },
      {
        name: 'streams',
        path: '/{proj_id}/streams',
        children: [
          {
            name: 'credentials',
            path: '/{stream_id}/credentials',
          },
          {
            name: 'message-log',
            path: '/{stream_id}/messages',
          },
        ]
      }
    ]
  },
  {
    name: 'domains',
    path: '/domains',
  },
  {
    name: 'organizations',
    path: '/organizations',
    children: [
      {
        name: 'add',
        path: '/add',
      },
      {
        name: 'edit',
        path: '/edit/{id}',
      }
    ]
  },
];

export function matchPath(path: string): { route: Route, params: RouteParams } | null {
  return matchPathRecursive(path, routes, {})
}

// TODO remove logging

function matchPathRecursive(path: string, routes: Route[], pathParams: { [k: string]: string }): {
  route: Route,
  params: RouteParams
} | null {
  const new_path_params: { [k: string]: string } = {};
  path = path.replace(/^\/|\/$/, '');
  console.log('path', path)
  const path_elems = path.split('/');
  console.log('path_elems', path_elems)
  route_loop:
    for (const route of routes) {
      const route_path = route.path.replace(/^\/|\/$/, '')
      const route_elems = route_path.split('/');
      console.log('route_elems', route_elems)
      for (const [index, route_elem] of route_elems.entries()) {
        console.log('try matching', route_elem, path_elems[index]);
        if (route_elem !== path_elems[index]) {
          const path_var = route_elem.match(/^{(\w*)}$/)?.at(1);
          if (path_var) {
            console.log('matched path_var', path_var)
            new_path_params[path_var] = path_elems[index];
          } else {
            console.log('no match')
            continue route_loop;
          }
        }
        console.log('matched segment', route_elem, path_elems[index])
      }
      if (route.children) {
        const rec_res = matchPathRecursive(path_elems.slice(route_elems.length).join('/'), route.children, new_path_params);
        console.log('rec_res', rec_res)
        if (rec_res) {
          return {route: rec_res.route, params: {...pathParams, ...rec_res.params, ...new_path_params}}
        } else {
          return null;
        }
      } else {
        return {route, params: {...pathParams, ...new_path_params}};
      }
    }
  return null;
}

export function matchName(name: RouteName): { route: Route, fullPath: string } {
  const elems = name.split('.');
  return recursiveMatchName(routes, elems, '') || {route: routes[0], fullPath: routes[0].path};
}

function recursiveMatchName(routes: Route[], name_elems: string[], fullPath: string): {
  route: Route,
  fullPath: string
} | null {
  const elem = name_elems.at(0);
  if (!elem) {
    console.log('No elems')
    return null
  }
  for (const route of routes) {
    console.log('matching', route.name, elem);
    if (route.name === elem) {
      if (name_elems.length > 1 && route.children) {
        console.log('recursing', route.name, name_elems.slice(1))
        return recursiveMatchName(route.children, name_elems.slice(1), `${fullPath}/${route.path.replace(/^\/|\/$/, '')}`)
      }
      console.log('matched', route, `${fullPath}/${route.path.replace(/^\/|\/$/, '')}`)
      return {route, fullPath: `${fullPath}/${route.path.replace(/^\/|\/$/, '')}`};
    }
  }
  console.log('no match')
  return null
}

export const RouterContext = createContext<RouterContextProps>({
  route: routes[0],
  fullPath: routes[0].path,
  params: {},
  navigate: () => {
  },
});

export function useRouter(): RouterContextProps {
  return useContext(RouterContext);
}

export function useInitRouter(): RouterContextProps {
  const currentPath = window.location.pathname;
  const {route: init_route, params: path_params} = matchPath(currentPath) || {route: routes[0], params: {}};
  const [route, setRoute] = useState<{ route: Route, fullPath: string }>({route: init_route, fullPath: currentPath});

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
        setRoute({route: init_route, fullPath: currentPath});
        setParams({...Object.fromEntries(queryString), ...path_params});
      }
      if (event.state?.routeParams) {
        setParams(event.state.routeParams);
      }
    });
  }, []);

  // navigate to a new route
  const navigate = (name: RouteName, params: RouteParams = {}) => {
    // eslint-disable-next-line prefer-const
    let {route: newRoute, fullPath: path} = matchName(name);
    console.log('newRoute', newRoute);

    if (params && Object.keys(params).length > 0) {
      const queryParams: { [k: string]: string } = {};
      for (const key of Object.keys(params)) {
        console.log(key, params[key])
        if (path.includes(`{${key}}`)) {
          path = path.replace(`{${key}}`, params[key])
        } else {
          queryParams[key] = params[key];
        }
      }
      const searchParams = new URLSearchParams(queryParams);
      const fullPath = Object.keys(queryParams).length === 0 ? path : `${path}?${searchParams}`;
      console.log('fullPath', fullPath);
      window.history.pushState(
        {routeName: name, routeParams: params},
        '',
        fullPath
      );
      setRoute({route: newRoute, fullPath: path});
      setParams(params);
    } else {
      window.history.pushState(
        {routeName: name, routeParams: {}},
        '',
        path
      );
      setRoute({route: newRoute, fullPath: path});
      setParams({});
    }
  };

  return {
    params,
    route: route.route,
    fullPath: route.fullPath,
    navigate,
  };
}