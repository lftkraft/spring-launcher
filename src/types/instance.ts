export type LoaderType = 'vanilla' | 'forge' | 'fabric' | 'quilt' | 'neoforge';

export interface Instance {
  id: string;
  name: string;
  mcVersion: string;
  loader: LoaderType;
  loaderVersion?: string;
  memory: number;
  javaPath?: string;
  gameDir: string;
  created: string;
  lastPlayed?: string;
  icon?: string;
  playtime?: number; // Másodpercben
}