import { BrowserRouter, Navigate, Outlet, Route, Routes } from 'react-router-dom';
import { AssetsPage } from '../features/assets/AssetsPage';
import { LoginPage } from '../features/auth/LoginPage';
import { RootRedirect, RequireAuth } from '../features/auth/RequireAuth';
import { AgentDetailsPage } from '../features/agents/AgentDetailsPage';
import { AgentsPage } from '../features/agents/AgentsPage';
import {
  CreateDocumentPage,
  DocumentsIndexPage,
  DocumentViewPage,
  EditDocumentPage,
} from '../features/documents/DocumentsPage';
import { ProjectOverviewPage } from '../features/projects/ProjectOverviewPage';
import { ProjectRouteLayout } from '../features/projects/ProjectRouteLayout';
import { WorkspacePage } from '../features/projects/WorkspacePage';
import { IntegrationConnectionsPage } from '../features/integrations/IntegrationConnectionsPage';
import { NotesPage } from '../features/notes/NotesPage';
import { TasksPage } from '../features/tasks/TasksPage';
import { WorkspacesPage } from '../features/workspaces/WorkspacesPage';
import { AppFrame } from '../shared/ui/AppFrame';

function ProtectedAppLayout() {
  return (
    <RequireAuth>
      <AppFrame>
        <Outlet />
      </AppFrame>
    </RequireAuth>
  );
}

export function AppRouter() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<RootRedirect />} />
        <Route path="/login" element={<LoginPage />} />
        <Route element={<ProtectedAppLayout />}>
          <Route path="/workspaces" element={<WorkspacesPage />} />
          <Route path="/workspaces/:workspaceSlug" element={<WorkspacePage />} />
          <Route path="/workspaces/:workspaceSlug/agents" element={<AgentsPage />} />
          <Route path="/workspaces/:workspaceSlug/agents/:agentId" element={<AgentDetailsPage />} />
          <Route path="/workspaces/:workspaceSlug/integrations" element={<IntegrationConnectionsPage />} />
          <Route path="/workspaces/:workspaceSlug/projects/:projectSlug" element={<ProjectRouteLayout />}>
            <Route index element={<ProjectOverviewPage />} />
            <Route path="tasks" element={<TasksPage />} />
            <Route path="documents" element={<DocumentsIndexPage />} />
            <Route path="documents/new" element={<CreateDocumentPage />} />
            <Route path="documents/:documentId" element={<DocumentViewPage />} />
            <Route path="documents/:documentId/edit" element={<EditDocumentPage />} />
            <Route path="notes" element={<NotesPage />} />
            <Route path="assets" element={<AssetsPage />} />
          </Route>
        </Route>
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </BrowserRouter>
  );
}
