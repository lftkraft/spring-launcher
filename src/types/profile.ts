export interface Profile {
  id: string;
  name: string;
  uuid: string;
  profile_type: 'offline' | 'microsoft';
  access_token: string;
  skinUrl?: string;
}