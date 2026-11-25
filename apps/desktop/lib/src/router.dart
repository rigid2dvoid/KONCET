import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_ui/material_ui.dart';

import 'screens/history_screen.dart';
import 'screens/new_project_screen.dart';
import 'screens/project_detail_screen.dart';
import 'screens/projects_screen.dart';
import 'screens/run_workspace_screen.dart';
import 'screens/scenario_editor_screen.dart';
import 'screens/settings_screen.dart';
import 'screens/source_editor_screen.dart';
import 'shell.dart';

final rootNavigatorKey = GlobalKey<NavigatorState>();

final routerProvider = Provider<GoRouter>((ref) {
  return GoRouter(
    navigatorKey: rootNavigatorKey,
    initialLocation: '/projects',
    routes: [
      GoRoute(path: '/', redirect: (context, state) => '/projects'),
      ShellRoute(
        builder: (context, state, child) =>
            AppShell(location: state.uri.path, child: child),
        routes: [
          GoRoute(
            path: '/projects',
            pageBuilder: (context, state) =>
                const NoTransitionPage(child: ProjectsScreen()),
          ),
          GoRoute(
            path: '/projects/new',
            pageBuilder: (context, state) =>
                const NoTransitionPage(child: NewProjectScreen()),
          ),
          GoRoute(
            path: '/projects/:projectId',
            pageBuilder: (context, state) => NoTransitionPage(
              child: ProjectDetailScreen(
                projectId: state.pathParameters['projectId']!,
              ),
            ),
          ),
          GoRoute(
            path: '/projects/:projectId/source',
            pageBuilder: (context, state) => NoTransitionPage(
              child: SourceEditorScreen(
                projectId: state.pathParameters['projectId']!,
              ),
            ),
          ),
          GoRoute(
            path: '/projects/:projectId/scenarios/:scenarioId',
            pageBuilder: (context, state) => NoTransitionPage(
              child: ScenarioEditorScreen(
                projectId: state.pathParameters['projectId']!,
                scenarioId: state.pathParameters['scenarioId']!,
              ),
            ),
          ),
          GoRoute(
            path: '/runs',
            pageBuilder: (context, state) =>
                const NoTransitionPage(child: HistoryScreen()),
          ),
          GoRoute(
            path: '/runs/:runId',
            pageBuilder: (context, state) => NoTransitionPage(
              child: RunWorkspaceScreen(runId: state.pathParameters['runId']!),
            ),
          ),
          GoRoute(
            path: '/settings',
            pageBuilder: (context, state) =>
                const NoTransitionPage(child: SettingsScreen()),
          ),
        ],
      ),
    ],
  );
});
