export interface RecentServer {
  id: string;
  server_address: string;
  server_name: string;
  instance_id: string;
  instance_name: string;
  instance_loader: string;
  instance_version: string;
  instance_icon?: string;
  icon?: string;
  last_played?: string;
}
