use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::models::*;

const LINEAR_API_URL: &str = "https://api.linear.app/graphql";

/// Linear API client
pub struct LinearClient {
    client: Client,
    api_key: String,
}

#[derive(Debug, Deserialize)]
struct GraphQLResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: String,
}

impl LinearClient {
    pub fn new(api_key: &str) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            api_key: api_key.to_string(),
        })
    }

    async fn execute<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        variables: Option<Value>,
    ) -> Result<T> {
        let body = json!({
            "query": query,
            "variables": variables.unwrap_or(json!({}))
        });

        let response = self
            .client
            .post(LINEAR_API_URL)
            .header("Authorization", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to send request to Linear API")?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            return Err(anyhow!("Linear API error ({}): {}", status, text));
        }

        let response: GraphQLResponse<T> =
            serde_json::from_str(&text).context("Failed to parse Linear API response")?;

        if let Some(errors) = response.errors {
            let messages: Vec<_> = errors.iter().map(|e| e.message.as_str()).collect();
            return Err(anyhow!("GraphQL errors: {}", messages.join(", ")));
        }

        response.data.ok_or_else(|| anyhow!("No data in response"))
    }

    /// Fetch all teams
    pub async fn get_teams(&self) -> Result<Vec<Team>> {
        #[derive(Deserialize)]
        struct TeamsData {
            teams: Nodes<TeamNode>,
        }

        #[derive(Deserialize)]
        struct Nodes<T> {
            nodes: Vec<T>,
        }

        #[derive(Deserialize)]
        struct TeamNode {
            id: String,
            name: String,
            key: String,
            icon: Option<String>,
        }

        let query = r#"
            query {
                teams {
                    nodes {
                        id
                        name
                        key
                        icon
                    }
                }
            }
        "#;

        let data: TeamsData = self.execute(query, None).await?;

        Ok(data
            .teams
            .nodes
            .into_iter()
            .map(|t| Team {
                id: t.id,
                name: t.name,
                key: t.key,
                icon: t.icon,
            })
            .collect())
    }

    /// Fetch projects for a team
    pub async fn get_projects(&self, team_id: Option<&str>) -> Result<Vec<Project>> {
        #[derive(Deserialize)]
        struct ProjectsData {
            projects: Nodes<ProjectNode>,
        }

        #[derive(Deserialize)]
        struct Nodes<T> {
            nodes: Vec<T>,
        }

        #[derive(Deserialize)]
        struct ProjectNode {
            id: String,
            name: String,
            description: Option<String>,
            icon: Option<String>,
            color: Option<String>,
            #[serde(rename = "startDate")]
            start_date: Option<String>,
            #[serde(rename = "targetDate")]
            target_date: Option<String>,
        }

        let query = r#"
            query($teamId: String) {
                projects(
                    first: 100
                    filter: { 
                        state: { type: { nin: ["canceled"] } }
                        team: { id: { eq: $teamId } }
                    }
                ) {
                    nodes {
                        id
                        name
                        description
                        icon
                        color
                        startDate
                        targetDate
                    }
                }
            }
        "#;

        let variables = team_id.map(|id| json!({ "teamId": id }));
        let data: ProjectsData = self.execute(query, variables).await?;

        Ok(data
            .projects
            .nodes
            .into_iter()
            .map(|p| Project {
                id: p.id,
                name: p.name,
                description: p.description,
                icon: p.icon,
                color: p.color,
                start_date: p.start_date.and_then(|d| d.parse().ok()),
                target_date: p.target_date.and_then(|d| d.parse().ok()),
            })
            .collect())
    }

    /// Fetch issues/tasks with optional filters
    pub async fn get_tasks(
        &self,
        team_id: Option<&str>,
        project_id: Option<&str>,
        label_name: Option<&str>,
        include_completed: bool,
    ) -> Result<Vec<Task>> {
        #[derive(Deserialize)]
        struct IssuesData {
            issues: Nodes<IssueNode>,
        }

        #[derive(Deserialize)]
        struct Nodes<T> {
            nodes: Vec<T>,
        }

        #[derive(Deserialize)]
        struct IssueNode {
            id: String,
            identifier: String,
            title: String,
            description: Option<String>,
            priority: i32,
            #[serde(rename = "dueDate")]
            due_date: Option<String>,
            state: StateNode,
            labels: Nodes<LabelNode>,
            project: Option<ProjectNode>,
            assignee: Option<UserNode>,
            #[serde(rename = "createdAt")]
            created_at: String,
            #[serde(rename = "updatedAt")]
            updated_at: String,
            url: String,
        }

        #[derive(Deserialize)]
        struct StateNode {
            id: String,
            name: String,
            #[serde(rename = "type")]
            state_type: String,
            color: Option<String>,
        }

        #[derive(Deserialize)]
        struct LabelNode {
            id: String,
            name: String,
            color: Option<String>,
        }

        #[derive(Deserialize)]
        struct ProjectNode {
            id: String,
            name: String,
            description: Option<String>,
            icon: Option<String>,
            color: Option<String>,
        }

        #[derive(Deserialize)]
        struct UserNode {
            id: String,
            name: String,
            email: Option<String>,
        }

        let state_filter = if include_completed {
            json!({})
        } else {
            json!({ "type": { "nin": ["completed", "canceled"] } })
        };

        let mut filter = json!({
            "state": state_filter
        });

        if let Some(tid) = team_id {
            filter["team"] = json!({ "id": { "eq": tid } });
        }

        if let Some(pid) = project_id {
            filter["project"] = json!({ "id": { "eq": pid } });
        }

        if let Some(label) = label_name {
            filter["labels"] = json!({ "name": { "containsIgnoreCase": label } });
        }

        let query = r#"
            query($filter: IssueFilter) {
                issues(first: 100, filter: $filter, orderBy: updatedAt) {
                    nodes {
                        id
                        identifier
                        title
                        description
                        priority
                        dueDate
                        state {
                            id
                            name
                            type
                            color
                        }
                        labels {
                            nodes {
                                id
                                name
                                color
                            }
                        }
                        project {
                            id
                            name
                            description
                            icon
                            color
                        }
                        assignee {
                            id
                            name
                            email
                        }
                        createdAt
                        updatedAt
                        url
                    }
                }
            }
        "#;

        let variables = json!({ "filter": filter });
        let data: IssuesData = self.execute(query, Some(variables)).await?;

        Ok(data
            .issues
            .nodes
            .into_iter()
            .map(|i| Task {
                id: i.id,
                identifier: i.identifier,
                title: i.title,
                description: i.description,
                priority: i.priority,
                due_date: i.due_date.and_then(|d| d.parse().ok()),
                state: TaskState {
                    id: i.state.id,
                    name: i.state.name,
                    state_type: i.state.state_type,
                    color: i.state.color,
                },
                labels: i
                    .labels
                    .nodes
                    .into_iter()
                    .map(|l| Label {
                        id: l.id,
                        name: l.name,
                        color: l.color,
                    })
                    .collect(),
                project: i.project.map(|p| Project {
                    id: p.id,
                    name: p.name,
                    description: p.description,
                    icon: p.icon,
                    color: p.color,
                    start_date: None,
                    target_date: None,
                }),
                assignee: i.assignee.map(|u| User {
                    id: u.id,
                    name: u.name,
                    email: u.email,
                }),
                created_at: i.created_at.parse().unwrap_or_else(|_| chrono::Utc::now()),
                updated_at: i.updated_at.parse().unwrap_or_else(|_| chrono::Utc::now()),
                url: i.url,
            })
            .collect())
    }

    /// Get workflow states for a team
    pub async fn get_states(&self, team_id: &str) -> Result<Vec<TaskState>> {
        #[derive(Deserialize)]
        struct StatesData {
            workflowStates: Nodes<StateNode>,
        }

        #[derive(Deserialize)]
        struct Nodes<T> {
            nodes: Vec<T>,
        }

        #[derive(Deserialize)]
        struct StateNode {
            id: String,
            name: String,
            #[serde(rename = "type")]
            state_type: String,
            color: Option<String>,
        }

        let query = r#"
            query($teamId: String!) {
                workflowStates(filter: { team: { id: { eq: $teamId } } }) {
                    nodes {
                        id
                        name
                        type
                        color
                    }
                }
            }
        "#;

        let variables = json!({ "teamId": team_id });
        let data: StatesData = self.execute(query, Some(variables)).await?;

        Ok(data
            .workflowStates
            .nodes
            .into_iter()
            .map(|s| TaskState {
                id: s.id,
                name: s.name,
                state_type: s.state_type,
                color: s.color,
            })
            .collect())
    }

    /// Get labels for a team
    pub async fn get_labels(&self, team_id: Option<&str>) -> Result<Vec<Label>> {
        #[derive(Deserialize)]
        struct LabelsData {
            issueLabels: Nodes<LabelNode>,
        }

        #[derive(Deserialize)]
        struct Nodes<T> {
            nodes: Vec<T>,
        }

        #[derive(Deserialize)]
        struct LabelNode {
            id: String,
            name: String,
            color: Option<String>,
        }

        let query = if team_id.is_some() {
            r#"
                query($teamId: String!) {
                    issueLabels(filter: { team: { id: { eq: $teamId } } }) {
                        nodes {
                            id
                            name
                            color
                        }
                    }
                }
            "#
        } else {
            r#"
                query {
                    issueLabels {
                        nodes {
                            id
                            name
                            color
                        }
                    }
                }
            "#
        };

        let variables = team_id.map(|id| json!({ "teamId": id }));
        let data: LabelsData = self.execute(query, variables).await?;

        Ok(data
            .issueLabels
            .nodes
            .into_iter()
            .map(|l| Label {
                id: l.id,
                name: l.name,
                color: l.color,
            })
            .collect())
    }

    /// Create a new task
    pub async fn create_task(&self, input: CreateTaskInput) -> Result<Task> {
        #[derive(Deserialize)]
        struct CreateData {
            issueCreate: IssuePayload,
        }

        #[derive(Deserialize)]
        struct IssuePayload {
            success: bool,
            issue: Option<IssueNode>,
        }

        #[derive(Deserialize)]
        struct IssueNode {
            id: String,
            identifier: String,
            title: String,
            url: String,
        }

        let mut issue_input = json!({
            "title": input.title,
            "teamId": input.team_id,
        });

        if let Some(desc) = &input.description {
            issue_input["description"] = json!(desc);
        }
        if let Some(project_id) = &input.project_id {
            issue_input["projectId"] = json!(project_id);
        }
        if let Some(priority) = input.priority {
            issue_input["priority"] = json!(priority);
        }
        if let Some(due) = &input.due_date {
            issue_input["dueDate"] = json!(due.to_string());
        }
        if !input.label_ids.is_empty() {
            issue_input["labelIds"] = json!(input.label_ids);
        }
        if let Some(assignee) = &input.assignee_id {
            issue_input["assigneeId"] = json!(assignee);
        }

        let query = r#"
            mutation($input: IssueCreateInput!) {
                issueCreate(input: $input) {
                    success
                    issue {
                        id
                        identifier
                        title
                        url
                    }
                }
            }
        "#;

        let variables = json!({ "input": issue_input });
        let data: CreateData = self.execute(query, Some(variables)).await?;

        if !data.issueCreate.success {
            return Err(anyhow!("Failed to create task"));
        }

        let issue = data
            .issueCreate
            .issue
            .ok_or_else(|| anyhow!("No issue returned"))?;

        // Fetch full task details
        self.get_task(&issue.id).await
    }

    /// Get a single task by ID
    pub async fn get_task(&self, id: &str) -> Result<Task> {
        #[derive(Deserialize)]
        struct IssueData {
            issue: IssueNode,
        }

        #[derive(Deserialize)]
        struct IssueNode {
            id: String,
            identifier: String,
            title: String,
            description: Option<String>,
            priority: i32,
            #[serde(rename = "dueDate")]
            due_date: Option<String>,
            state: StateNode,
            labels: Nodes<LabelNode>,
            project: Option<ProjectNode>,
            assignee: Option<UserNode>,
            #[serde(rename = "createdAt")]
            created_at: String,
            #[serde(rename = "updatedAt")]
            updated_at: String,
            url: String,
        }

        #[derive(Deserialize)]
        struct StateNode {
            id: String,
            name: String,
            #[serde(rename = "type")]
            state_type: String,
            color: Option<String>,
        }

        #[derive(Deserialize)]
        struct Nodes<T> {
            nodes: Vec<T>,
        }

        #[derive(Deserialize)]
        struct LabelNode {
            id: String,
            name: String,
            color: Option<String>,
        }

        #[derive(Deserialize)]
        struct ProjectNode {
            id: String,
            name: String,
            description: Option<String>,
            icon: Option<String>,
            color: Option<String>,
        }

        #[derive(Deserialize)]
        struct UserNode {
            id: String,
            name: String,
            email: Option<String>,
        }

        let query = r#"
            query($id: String!) {
                issue(id: $id) {
                    id
                    identifier
                    title
                    description
                    priority
                    dueDate
                    state {
                        id
                        name
                        type
                        color
                    }
                    labels {
                        nodes {
                            id
                            name
                            color
                        }
                    }
                    project {
                        id
                        name
                        description
                        icon
                        color
                    }
                    assignee {
                        id
                        name
                        email
                    }
                    createdAt
                    updatedAt
                    url
                }
            }
        "#;

        let variables = json!({ "id": id });
        let data: IssueData = self.execute(query, Some(variables)).await?;

        let i = data.issue;
        Ok(Task {
            id: i.id,
            identifier: i.identifier,
            title: i.title,
            description: i.description,
            priority: i.priority,
            due_date: i.due_date.and_then(|d| d.parse().ok()),
            state: TaskState {
                id: i.state.id,
                name: i.state.name,
                state_type: i.state.state_type,
                color: i.state.color,
            },
            labels: i
                .labels
                .nodes
                .into_iter()
                .map(|l| Label {
                    id: l.id,
                    name: l.name,
                    color: l.color,
                })
                .collect(),
            project: i.project.map(|p| Project {
                id: p.id,
                name: p.name,
                description: p.description,
                icon: p.icon,
                color: p.color,
                start_date: None,
                target_date: None,
            }),
            assignee: i.assignee.map(|u| User {
                id: u.id,
                name: u.name,
                email: u.email,
            }),
            created_at: i.created_at.parse().unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: i.updated_at.parse().unwrap_or_else(|_| chrono::Utc::now()),
            url: i.url,
        })
    }

    /// Update a task
    pub async fn update_task(&self, input: UpdateTaskInput) -> Result<Task> {
        #[derive(Deserialize)]
        struct UpdateData {
            issueUpdate: IssuePayload,
        }

        #[derive(Deserialize)]
        struct IssuePayload {
            success: bool,
        }

        let mut issue_input = json!({});

        if let Some(title) = &input.title {
            issue_input["title"] = json!(title);
        }
        if let Some(desc) = &input.description {
            issue_input["description"] = json!(desc);
        }
        if let Some(priority) = input.priority {
            issue_input["priority"] = json!(priority);
        }
        if let Some(due) = &input.due_date {
            issue_input["dueDate"] = json!(due.to_string());
        }
        if let Some(state_id) = &input.state_id {
            issue_input["stateId"] = json!(state_id);
        }
        if let Some(label_ids) = &input.label_ids {
            issue_input["labelIds"] = json!(label_ids);
        }
        if let Some(project_id) = &input.project_id {
            issue_input["projectId"] = json!(project_id);
        }

        let query = r#"
            mutation($id: String!, $input: IssueUpdateInput!) {
                issueUpdate(id: $id, input: $input) {
                    success
                }
            }
        "#;

        let variables = json!({
            "id": input.id,
            "input": issue_input
        });

        let data: UpdateData = self.execute(query, Some(variables)).await?;

        if !data.issueUpdate.success {
            return Err(anyhow!("Failed to update task"));
        }

        self.get_task(&input.id).await
    }

    /// Delete/archive a task
    pub async fn delete_task(&self, id: &str) -> Result<bool> {
        #[derive(Deserialize)]
        struct DeleteData {
            issueArchive: ArchivePayload,
        }

        #[derive(Deserialize)]
        struct ArchivePayload {
            success: bool,
        }

        let query = r#"
            mutation($id: String!) {
                issueArchive(id: $id) {
                    success
                }
            }
        "#;

        let variables = json!({ "id": id });
        let data: DeleteData = self.execute(query, Some(variables)).await?;

        Ok(data.issueArchive.success)
    }

    /// Create a label
    pub async fn create_label(&self, name: &str, color: &str, team_id: Option<&str>) -> Result<Label> {
        #[derive(Deserialize)]
        struct CreateData {
            issueLabelCreate: LabelPayload,
        }

        #[derive(Deserialize)]
        struct LabelPayload {
            success: bool,
            issueLabel: Option<LabelNode>,
        }

        #[derive(Deserialize)]
        struct LabelNode {
            id: String,
            name: String,
            color: Option<String>,
        }

        let mut input = json!({
            "name": name,
            "color": color
        });

        if let Some(tid) = team_id {
            input["teamId"] = json!(tid);
        }

        let query = r#"
            mutation($input: IssueLabelCreateInput!) {
                issueLabelCreate(input: $input) {
                    success
                    issueLabel {
                        id
                        name
                        color
                    }
                }
            }
        "#;

        let variables = json!({ "input": input });
        let data: CreateData = self.execute(query, Some(variables)).await?;

        if !data.issueLabelCreate.success {
            return Err(anyhow!("Failed to create label"));
        }

        let label = data
            .issueLabelCreate
            .issueLabel
            .ok_or_else(|| anyhow!("No label returned"))?;

        Ok(Label {
            id: label.id,
            name: label.name,
            color: label.color,
        })
    }
}
