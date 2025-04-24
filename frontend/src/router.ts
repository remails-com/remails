import {BreadcrumbItem} from "./types.ts";

export type RouteName = string;
export type RouteParams = Record<string, string>;
export type Navigate = (name: RouteName, params?: RouteParams) => void;

export interface Route {
  name: RouteName;
  path: string;
  display: string;
  children?: Route[];
}

export const routes: Route[] = [
  {
    name: 'projects',
    display: 'Projects',
    path: '/{org_id}/projects',
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

export function routerNavigate(name: RouteName, params: RouteParams): {
  route: Route,
  fullPath: string,
  fullName: string,
  params: { [k: string]: string },
  breadcrumbItems: BreadcrumbItem[]
} {
  let {route: newRoute, fullPath: path, breadcrumbItems, fullName} = matchName(name);

  const queryParams: { [k: string]: string } = {...params};

  path = path.replace(/{(\w*)}/g, (_match, path_var) => {
    const path = params[path_var] || `{${path_var}}`;
    delete queryParams[path_var]
    return path;
  })

  if (params && Object.keys(params).length > 0) {
    const searchParams = new URLSearchParams(queryParams);
    const fullPath = Object.keys(queryParams).length === 0 ? path : `${path}?${searchParams}`;
    window.history.pushState(
      {routeName: name, routeParams: params},
      '',
      fullPath
    );
    return {route: newRoute, fullPath: path, fullName, breadcrumbItems, params};
  } else {
    window.history.pushState(
      {routeName: name, routeParams: {}},
      '',
      path
    );
    return {route: newRoute, fullPath: path, fullName, breadcrumbItems, params: {}};
  }
}

export function initRouter() {
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
  const queryString = new URLSearchParams(window.location.search);
  const params = {...Object.fromEntries(queryString), ...path_params};

  return {
    route: init_route,
    fullPath: currentPath,
    fullName: initFullName,
    breadcrumbItems: path_breadcrumbItems,
    params
  };
}
