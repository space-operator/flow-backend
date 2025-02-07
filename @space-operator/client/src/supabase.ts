export type Json =
  | string
  | number
  | boolean
  | null
  | { [key: string]: Json | undefined }
  | Json[]

export type Database = {
  graphql_public: {
    Tables: {
      [_ in never]: never
    }
    Views: {
      [_ in never]: never
    }
    Functions: {
      graphql: {
        Args: {
          operationName?: string
          query?: string
          variables?: Json
          extensions?: Json
        }
        Returns: Json
      }
    }
    Enums: {
      [_ in never]: never
    }
    CompositeTypes: {
      [_ in never]: never
    }
  }
  pgbouncer: {
    Tables: {
      [_ in never]: never
    }
    Views: {
      [_ in never]: never
    }
    Functions: {
      get_auth: {
        Args: {
          p_usename: string
        }
        Returns: {
          username: string
          password: string
        }[]
      }
    }
    Enums: {
      [_ in never]: never
    }
    CompositeTypes: {
      [_ in never]: never
    }
  }
  public: {
    Tables: {
      apikeys: {
        Row: {
          created_at: string
          key_hash: string
          name: string
          trimmed_key: string
          user_id: string
        }
        Insert: {
          created_at?: string
          key_hash: string
          name: string
          trimmed_key: string
          user_id: string
        }
        Update: {
          created_at?: string
          key_hash?: string
          name?: string
          trimmed_key?: string
          user_id?: string
        }
        Relationships: [
          {
            foreignKeyName: "fk-user_id"
            columns: ["user_id"]
            referencedRelation: "users"
            referencedColumns: ["id"]
          },
        ]
      }
      flow_deployments: {
        Row: {
          action_identity: string | null
          created_at: string
          entrypoint: number
          fees: Json
          id: string
          output_instructions: boolean
          solana_network: Json
          start_permission: Json
          user_id: string
        }
        Insert: {
          action_identity?: string | null
          created_at?: string
          entrypoint: number
          fees: Json
          id: string
          output_instructions: boolean
          solana_network: Json
          start_permission: Json
          user_id: string
        }
        Update: {
          action_identity?: string | null
          created_at?: string
          entrypoint?: number
          fees?: Json
          id?: string
          output_instructions?: boolean
          solana_network?: Json
          start_permission?: Json
          user_id?: string
        }
        Relationships: []
      }
      flow_deployments_flows: {
        Row: {
          data: Json
          deployment_id: string
          flow_id: number
          user_id: string
        }
        Insert: {
          data: Json
          deployment_id: string
          flow_id: number
          user_id: string
        }
        Update: {
          data?: Json
          deployment_id?: string
          flow_id?: number
          user_id?: string
        }
        Relationships: [
          {
            foreignKeyName: "flow_deployments_flows_deployment_id_fkey"
            columns: ["deployment_id"]
            referencedRelation: "flow_deployments"
            referencedColumns: ["id"]
          },
          {
            foreignKeyName: "flow_deployments_flows_user_id_fkey"
            columns: ["user_id"]
            referencedRelation: "users"
            referencedColumns: ["id"]
          },
        ]
      }
      flow_deployments_tags: {
        Row: {
          deployment_id: string
          description: string | null
          entrypoint: number
          tag: string
          user_id: string
        }
        Insert: {
          deployment_id: string
          description?: string | null
          entrypoint: number
          tag: string
          user_id: string
        }
        Update: {
          deployment_id?: string
          description?: string | null
          entrypoint?: number
          tag?: string
          user_id?: string
        }
        Relationships: [
          {
            foreignKeyName: "flow_deployments_tags_deployment_id_entrypoint_fkey"
            columns: ["deployment_id", "entrypoint"]
            referencedRelation: "flow_deployments"
            referencedColumns: ["id", "entrypoint"]
          },
          {
            foreignKeyName: "flow_deployments_tags_deployment_id_fkey"
            columns: ["deployment_id"]
            referencedRelation: "flow_deployments"
            referencedColumns: ["id"]
          },
          {
            foreignKeyName: "flow_deployments_tags_user_id_fkey"
            columns: ["user_id"]
            referencedRelation: "users"
            referencedColumns: ["id"]
          },
        ]
      }
      flow_deployments_wallets: {
        Row: {
          deployment_id: string
          user_id: string
          wallet_id: number
        }
        Insert: {
          deployment_id: string
          user_id: string
          wallet_id: number
        }
        Update: {
          deployment_id?: string
          user_id?: string
          wallet_id?: number
        }
        Relationships: [
          {
            foreignKeyName: "flow_deployments_wallets_deployment_id_fkey"
            columns: ["deployment_id"]
            referencedRelation: "flow_deployments"
            referencedColumns: ["id"]
          },
          {
            foreignKeyName: "flow_deployments_wallets_user_id_fkey"
            columns: ["user_id"]
            referencedRelation: "users"
            referencedColumns: ["id"]
          },
        ]
      }
      flow_run: {
        Row: {
          call_depth: number
          collect_instructions: boolean
          deployment_id: string | null
          edges: Json[]
          end_time: string | null
          environment: Json
          errors: string[] | null
          flow_id: number
          id: string
          inputs: Json
          instructions_bundling: Json
          network: Json
          nodes: Json[]
          not_run: string[] | null
          origin: Json
          output: Json | null
          partial_config: Json | null
          signers: Json
          start_time: string | null
          user_id: string
        }
        Insert: {
          call_depth: number
          collect_instructions: boolean
          deployment_id?: string | null
          edges: Json[]
          end_time?: string | null
          environment: Json
          errors?: string[] | null
          flow_id: number
          id: string
          inputs: Json
          instructions_bundling: Json
          network: Json
          nodes: Json[]
          not_run?: string[] | null
          origin: Json
          output?: Json | null
          partial_config?: Json | null
          signers: Json
          start_time?: string | null
          user_id: string
        }
        Update: {
          call_depth?: number
          collect_instructions?: boolean
          deployment_id?: string | null
          edges?: Json[]
          end_time?: string | null
          environment?: Json
          errors?: string[] | null
          flow_id?: number
          id?: string
          inputs?: Json
          instructions_bundling?: Json
          network?: Json
          nodes?: Json[]
          not_run?: string[] | null
          origin?: Json
          output?: Json | null
          partial_config?: Json | null
          signers?: Json
          start_time?: string | null
          user_id?: string
        }
        Relationships: [
          {
            foreignKeyName: "fk-flow_id"
            columns: ["flow_id"]
            referencedRelation: "flows"
            referencedColumns: ["id"]
          },
          {
            foreignKeyName: "fk-user_id"
            columns: ["user_id"]
            referencedRelation: "users"
            referencedColumns: ["id"]
          },
          {
            foreignKeyName: "flow_run_deployment_id_fkey"
            columns: ["deployment_id"]
            referencedRelation: "flow_deployments"
            referencedColumns: ["id"]
          },
        ]
      }
      flow_run_logs: {
        Row: {
          content: string
          flow_run_id: string
          log_index: number
          log_level: string
          module: string | null
          node_id: string | null
          time: string
          times: number | null
          user_id: string
        }
        Insert: {
          content: string
          flow_run_id: string
          log_index: number
          log_level: string
          module?: string | null
          node_id?: string | null
          time: string
          times?: number | null
          user_id: string
        }
        Update: {
          content?: string
          flow_run_id?: string
          log_index?: number
          log_level?: string
          module?: string | null
          node_id?: string | null
          time?: string
          times?: number | null
          user_id?: string
        }
        Relationships: [
          {
            foreignKeyName: "fk-flow_run_id"
            columns: ["flow_run_id"]
            referencedRelation: "flow_run"
            referencedColumns: ["id"]
          },
          {
            foreignKeyName: "fk-node_run_id"
            columns: ["flow_run_id", "node_id", "times"]
            referencedRelation: "node_run"
            referencedColumns: ["flow_run_id", "node_id", "times"]
          },
          {
            foreignKeyName: "fk-user_id"
            columns: ["user_id"]
            referencedRelation: "users"
            referencedColumns: ["id"]
          },
        ]
      }
      flow_run_shared: {
        Row: {
          flow_run_id: string
          user_id: string
        }
        Insert: {
          flow_run_id: string
          user_id: string
        }
        Update: {
          flow_run_id?: string
          user_id?: string
        }
        Relationships: [
          {
            foreignKeyName: "fk-flow_run_id"
            columns: ["flow_run_id"]
            referencedRelation: "flow_run"
            referencedColumns: ["id"]
          },
          {
            foreignKeyName: "fk-user_id"
            columns: ["user_id"]
            referencedRelation: "users"
            referencedColumns: ["id"]
          },
        ]
      }
      flows: {
        Row: {
          created_at: string
          current_network: Json
          custom_networks: Json[]
          description: string
          edges: Json[]
          environment: Json
          gg_marketplace: boolean | null
          guide: Json | null
          id: number
          instructions_bundling: Json
          isPublic: boolean
          lastest_flow_run_id: string | null
          mosaic: Json | null
          name: string
          nodes: Json[]
          parent_flow: number | null
          start_shared: boolean
          start_unverified: boolean
          tags: string[]
          updated_at: string | null
          user_id: string
          uuid: string | null
          viewport: Json
        }
        Insert: {
          created_at?: string
          current_network?: Json
          custom_networks?: Json[]
          description?: string
          edges?: Json[]
          environment?: Json
          gg_marketplace?: boolean | null
          guide?: Json | null
          id?: number
          instructions_bundling?: Json
          isPublic?: boolean
          lastest_flow_run_id?: string | null
          mosaic?: Json | null
          name?: string
          nodes?: Json[]
          parent_flow?: number | null
          start_shared?: boolean
          start_unverified?: boolean
          tags?: string[]
          updated_at?: string | null
          user_id?: string
          uuid?: string | null
          viewport?: Json
        }
        Update: {
          created_at?: string
          current_network?: Json
          custom_networks?: Json[]
          description?: string
          edges?: Json[]
          environment?: Json
          gg_marketplace?: boolean | null
          guide?: Json | null
          id?: number
          instructions_bundling?: Json
          isPublic?: boolean
          lastest_flow_run_id?: string | null
          mosaic?: Json | null
          name?: string
          nodes?: Json[]
          parent_flow?: number | null
          start_shared?: boolean
          start_unverified?: boolean
          tags?: string[]
          updated_at?: string | null
          user_id?: string
          uuid?: string | null
          viewport?: Json
        }
        Relationships: [
          {
            foreignKeyName: "flows_lastest_flow_run_id_fkey"
            columns: ["lastest_flow_run_id"]
            referencedRelation: "flow_run"
            referencedColumns: ["id"]
          },
          {
            foreignKeyName: "flows_user_id_fkey"
            columns: ["user_id"]
            referencedRelation: "users"
            referencedColumns: ["id"]
          },
        ]
      }
      kvstore: {
        Row: {
          key: string
          last_updated: string | null
          store_name: string
          user_id: string
          value: Json
        }
        Insert: {
          key: string
          last_updated?: string | null
          store_name: string
          user_id?: string
          value: Json
        }
        Update: {
          key?: string
          last_updated?: string | null
          store_name?: string
          user_id?: string
          value?: Json
        }
        Relationships: [
          {
            foreignKeyName: "kvstore_user_id_fkey"
            columns: ["user_id"]
            referencedRelation: "users"
            referencedColumns: ["id"]
          },
          {
            foreignKeyName: "kvstore_user_id_store_name_fkey"
            columns: ["user_id", "store_name"]
            referencedRelation: "kvstore_metadata"
            referencedColumns: ["user_id", "store_name"]
          },
        ]
      }
      kvstore_metadata: {
        Row: {
          stats_size: number
          store_name: string
          user_id: string
        }
        Insert: {
          stats_size?: number
          store_name: string
          user_id?: string
        }
        Update: {
          stats_size?: number
          store_name?: string
          user_id?: string
        }
        Relationships: [
          {
            foreignKeyName: "kvstore_metadata_user_id_fkey"
            columns: ["user_id"]
            referencedRelation: "users"
            referencedColumns: ["id"]
          },
          {
            foreignKeyName: "kvstore_metadata_user_id_user_quotas_fkey"
            columns: ["user_id"]
            referencedRelation: "user_quotas"
            referencedColumns: ["user_id"]
          },
        ]
      }
      node_run: {
        Row: {
          end_time: string | null
          errors: string[] | null
          flow_run_id: string
          input: Json
          node_id: string
          output: Json | null
          start_time: string | null
          times: number
          user_id: string
        }
        Insert: {
          end_time?: string | null
          errors?: string[] | null
          flow_run_id: string
          input?: Json
          node_id: string
          output?: Json | null
          start_time?: string | null
          times: number
          user_id: string
        }
        Update: {
          end_time?: string | null
          errors?: string[] | null
          flow_run_id?: string
          input?: Json
          node_id?: string
          output?: Json | null
          start_time?: string | null
          times?: number
          user_id?: string
        }
        Relationships: [
          {
            foreignKeyName: "fk-flow_run_id"
            columns: ["flow_run_id"]
            referencedRelation: "flow_run"
            referencedColumns: ["id"]
          },
          {
            foreignKeyName: "fk-user_id"
            columns: ["user_id"]
            referencedRelation: "users"
            referencedColumns: ["id"]
          },
        ]
      }
      nodes: {
        Row: {
          created_at: string | null
          data: Json
          id: number
          isPublic: boolean | null
          licenses: string[] | null
          name: string | null
          sources: Json
          status: string | null
          storage_path: string | null
          targets: Json
          "targets_form.extra": Json
          "targets_form.form_data": Json | null
          "targets_form.json_schema": Json | null
          "targets_form.ui_schema": Json | null
          type: string | null
          unique_node_id: string | null
          user_id: string | null
        }
        Insert: {
          created_at?: string | null
          data?: Json
          id?: number
          isPublic?: boolean | null
          licenses?: string[] | null
          name?: string | null
          sources?: Json
          status?: string | null
          storage_path?: string | null
          targets?: Json
          "targets_form.extra"?: Json
          "targets_form.form_data"?: Json | null
          "targets_form.json_schema"?: Json | null
          "targets_form.ui_schema"?: Json | null
          type?: string | null
          unique_node_id?: string | null
          user_id?: string | null
        }
        Update: {
          created_at?: string | null
          data?: Json
          id?: number
          isPublic?: boolean | null
          licenses?: string[] | null
          name?: string | null
          sources?: Json
          status?: string | null
          storage_path?: string | null
          targets?: Json
          "targets_form.extra"?: Json
          "targets_form.form_data"?: Json | null
          "targets_form.json_schema"?: Json | null
          "targets_form.ui_schema"?: Json | null
          type?: string | null
          unique_node_id?: string | null
          user_id?: string | null
        }
        Relationships: [
          {
            foreignKeyName: "nodes_user_id_fkey"
            columns: ["user_id"]
            referencedRelation: "users"
            referencedColumns: ["id"]
          },
        ]
      }
      pubkey_whitelists: {
        Row: {
          info: string | null
          pubkey: string
        }
        Insert: {
          info?: string | null
          pubkey: string
        }
        Update: {
          info?: string | null
          pubkey?: string
        }
        Relationships: []
      }
      signature_requests: {
        Row: {
          created_at: string
          flow_run_id: string | null
          id: number
          msg: string
          new_msg: string | null
          pubkey: string
          signature: string | null
          signatures: Json[] | null
          user_id: string
        }
        Insert: {
          created_at?: string
          flow_run_id?: string | null
          id?: number
          msg: string
          new_msg?: string | null
          pubkey: string
          signature?: string | null
          signatures?: Json[] | null
          user_id: string
        }
        Update: {
          created_at?: string
          flow_run_id?: string | null
          id?: number
          msg?: string
          new_msg?: string | null
          pubkey?: string
          signature?: string | null
          signatures?: Json[] | null
          user_id?: string
        }
        Relationships: [
          {
            foreignKeyName: "fk-user_id"
            columns: ["user_id"]
            referencedRelation: "users"
            referencedColumns: ["id"]
          },
          {
            foreignKeyName: "signature_requests_flow_run_id_fkey"
            columns: ["flow_run_id"]
            referencedRelation: "flow_run"
            referencedColumns: ["id"]
          },
        ]
      }
      user_quotas: {
        Row: {
          credit: number
          kvstore_count: number
          kvstore_count_limit: number
          kvstore_size: number
          kvstore_size_limit: number
          used_credit: number
          user_id: string
        }
        Insert: {
          credit?: number
          kvstore_count?: number
          kvstore_count_limit?: number
          kvstore_size?: number
          kvstore_size_limit?: number
          used_credit?: number
          user_id: string
        }
        Update: {
          credit?: number
          kvstore_count?: number
          kvstore_count_limit?: number
          kvstore_size?: number
          kvstore_size_limit?: number
          used_credit?: number
          user_id?: string
        }
        Relationships: [
          {
            foreignKeyName: "user_quotas_user_id_fkey"
            columns: ["user_id"]
            referencedRelation: "users"
            referencedColumns: ["id"]
          },
        ]
      }
      users_public: {
        Row: {
          avatar: string | null
          description: string | null
          email: string
          flow_skills: Json | null
          node_skills: Json | null
          pub_key: string
          status: string
          tasks_skills: Json | null
          updated_at: string | null
          user_id: string
          username: string | null
        }
        Insert: {
          avatar?: string | null
          description?: string | null
          email: string
          flow_skills?: Json | null
          node_skills?: Json | null
          pub_key: string
          status?: string
          tasks_skills?: Json | null
          updated_at?: string | null
          user_id: string
          username?: string | null
        }
        Update: {
          avatar?: string | null
          description?: string | null
          email?: string
          flow_skills?: Json | null
          node_skills?: Json | null
          pub_key?: string
          status?: string
          tasks_skills?: Json | null
          updated_at?: string | null
          user_id?: string
          username?: string | null
        }
        Relationships: [
          {
            foreignKeyName: "users_public_user_id_fkey"
            columns: ["user_id"]
            referencedRelation: "users"
            referencedColumns: ["id"]
          },
        ]
      }
      wallets: {
        Row: {
          adapter: string | null
          created_at: string | null
          description: string
          encrypted_keypair: Json | null
          icon: string | null
          id: number
          name: string
          public_key: string
          purpose: string | null
          type: string | null
          user_id: string
        }
        Insert: {
          adapter?: string | null
          created_at?: string | null
          description?: string
          encrypted_keypair?: Json | null
          icon?: string | null
          id?: number
          name?: string
          public_key: string
          purpose?: string | null
          type?: string | null
          user_id: string
        }
        Update: {
          adapter?: string | null
          created_at?: string | null
          description?: string
          encrypted_keypair?: Json | null
          icon?: string | null
          id?: number
          name?: string
          public_key?: string
          purpose?: string | null
          type?: string | null
          user_id?: string
        }
        Relationships: [
          {
            foreignKeyName: "wallets_user_id_fkey"
            columns: ["user_id"]
            referencedRelation: "users"
            referencedColumns: ["id"]
          },
        ]
      }
    }
    Views: {
      [_ in never]: never
    }
    Functions: {
      increase_credit: {
        Args: {
          user_id: string
          amount: number
        }
        Returns: number
      }
      increase_used_credit: {
        Args: {
          user_id: string
          amount: number
        }
        Returns: number
      }
      is_nft_admin: {
        Args: {
          user_id: string
        }
        Returns: boolean
      }
    }
    Enums: {
      [_ in never]: never
    }
    CompositeTypes: {
      [_ in never]: never
    }
  }
  storage: {
    Tables: {
      buckets: {
        Row: {
          allowed_mime_types: string[] | null
          avif_autodetection: boolean | null
          created_at: string | null
          file_size_limit: number | null
          id: string
          name: string
          owner: string | null
          owner_id: string | null
          public: boolean | null
          updated_at: string | null
        }
        Insert: {
          allowed_mime_types?: string[] | null
          avif_autodetection?: boolean | null
          created_at?: string | null
          file_size_limit?: number | null
          id: string
          name: string
          owner?: string | null
          owner_id?: string | null
          public?: boolean | null
          updated_at?: string | null
        }
        Update: {
          allowed_mime_types?: string[] | null
          avif_autodetection?: boolean | null
          created_at?: string | null
          file_size_limit?: number | null
          id?: string
          name?: string
          owner?: string | null
          owner_id?: string | null
          public?: boolean | null
          updated_at?: string | null
        }
        Relationships: []
      }
      migrations: {
        Row: {
          executed_at: string | null
          hash: string
          id: number
          name: string
        }
        Insert: {
          executed_at?: string | null
          hash: string
          id: number
          name: string
        }
        Update: {
          executed_at?: string | null
          hash?: string
          id?: number
          name?: string
        }
        Relationships: []
      }
      objects: {
        Row: {
          bucket_id: string | null
          created_at: string | null
          id: string
          last_accessed_at: string | null
          metadata: Json | null
          name: string | null
          owner: string | null
          owner_id: string | null
          path_tokens: string[] | null
          updated_at: string | null
          version: string | null
        }
        Insert: {
          bucket_id?: string | null
          created_at?: string | null
          id?: string
          last_accessed_at?: string | null
          metadata?: Json | null
          name?: string | null
          owner?: string | null
          owner_id?: string | null
          path_tokens?: string[] | null
          updated_at?: string | null
          version?: string | null
        }
        Update: {
          bucket_id?: string | null
          created_at?: string | null
          id?: string
          last_accessed_at?: string | null
          metadata?: Json | null
          name?: string | null
          owner?: string | null
          owner_id?: string | null
          path_tokens?: string[] | null
          updated_at?: string | null
          version?: string | null
        }
        Relationships: [
          {
            foreignKeyName: "objects_bucketId_fkey"
            columns: ["bucket_id"]
            referencedRelation: "buckets"
            referencedColumns: ["id"]
          },
        ]
      }
      s3_multipart_uploads: {
        Row: {
          bucket_id: string
          created_at: string
          id: string
          in_progress_size: number
          key: string
          owner_id: string | null
          upload_signature: string
          version: string
        }
        Insert: {
          bucket_id: string
          created_at?: string
          id: string
          in_progress_size?: number
          key: string
          owner_id?: string | null
          upload_signature: string
          version: string
        }
        Update: {
          bucket_id?: string
          created_at?: string
          id?: string
          in_progress_size?: number
          key?: string
          owner_id?: string | null
          upload_signature?: string
          version?: string
        }
        Relationships: [
          {
            foreignKeyName: "s3_multipart_uploads_bucket_id_fkey"
            columns: ["bucket_id"]
            referencedRelation: "buckets"
            referencedColumns: ["id"]
          },
        ]
      }
      s3_multipart_uploads_parts: {
        Row: {
          bucket_id: string
          created_at: string
          etag: string
          id: string
          key: string
          owner_id: string | null
          part_number: number
          size: number
          upload_id: string
          version: string
        }
        Insert: {
          bucket_id: string
          created_at?: string
          etag: string
          id?: string
          key: string
          owner_id?: string | null
          part_number: number
          size?: number
          upload_id: string
          version: string
        }
        Update: {
          bucket_id?: string
          created_at?: string
          etag?: string
          id?: string
          key?: string
          owner_id?: string | null
          part_number?: number
          size?: number
          upload_id?: string
          version?: string
        }
        Relationships: [
          {
            foreignKeyName: "s3_multipart_uploads_parts_bucket_id_fkey"
            columns: ["bucket_id"]
            referencedRelation: "buckets"
            referencedColumns: ["id"]
          },
          {
            foreignKeyName: "s3_multipart_uploads_parts_upload_id_fkey"
            columns: ["upload_id"]
            referencedRelation: "s3_multipart_uploads"
            referencedColumns: ["id"]
          },
        ]
      }
    }
    Views: {
      [_ in never]: never
    }
    Functions: {
      can_insert_object: {
        Args: {
          bucketid: string
          name: string
          owner: string
          metadata: Json
        }
        Returns: undefined
      }
      extension: {
        Args: {
          name: string
        }
        Returns: string
      }
      filename: {
        Args: {
          name: string
        }
        Returns: string
      }
      foldername: {
        Args: {
          name: string
        }
        Returns: string[]
      }
      get_size_by_bucket: {
        Args: Record<PropertyKey, never>
        Returns: {
          size: number
          bucket_id: string
        }[]
      }
      list_multipart_uploads_with_delimiter: {
        Args: {
          bucket_id: string
          prefix_param: string
          delimiter_param: string
          max_keys?: number
          next_key_token?: string
          next_upload_token?: string
        }
        Returns: {
          key: string
          id: string
          created_at: string
        }[]
      }
      list_objects_with_delimiter: {
        Args: {
          bucket_id: string
          prefix_param: string
          delimiter_param: string
          max_keys?: number
          start_after?: string
          next_token?: string
        }
        Returns: {
          name: string
          id: string
          metadata: Json
          updated_at: string
        }[]
      }
      search: {
        Args: {
          prefix: string
          bucketname: string
          limits?: number
          levels?: number
          offsets?: number
          search?: string
          sortcolumn?: string
          sortorder?: string
        }
        Returns: {
          name: string
          id: string
          updated_at: string
          created_at: string
          last_accessed_at: string
          metadata: Json
        }[]
      }
    }
    Enums: {
      [_ in never]: never
    }
    CompositeTypes: {
      [_ in never]: never
    }
  }
}

type PublicSchema = Database[Extract<keyof Database, "public">]

export type Tables<
  PublicTableNameOrOptions extends
    | keyof (PublicSchema["Tables"] & PublicSchema["Views"])
    | { schema: keyof Database },
  TableName extends PublicTableNameOrOptions extends { schema: keyof Database }
    ? keyof (Database[PublicTableNameOrOptions["schema"]]["Tables"] &
        Database[PublicTableNameOrOptions["schema"]]["Views"])
    : never = never,
> = PublicTableNameOrOptions extends { schema: keyof Database }
  ? (Database[PublicTableNameOrOptions["schema"]]["Tables"] &
      Database[PublicTableNameOrOptions["schema"]]["Views"])[TableName] extends {
      Row: infer R
    }
    ? R
    : never
  : PublicTableNameOrOptions extends keyof (PublicSchema["Tables"] &
        PublicSchema["Views"])
    ? (PublicSchema["Tables"] &
        PublicSchema["Views"])[PublicTableNameOrOptions] extends {
        Row: infer R
      }
      ? R
      : never
    : never

export type TablesInsert<
  PublicTableNameOrOptions extends
    | keyof PublicSchema["Tables"]
    | { schema: keyof Database },
  TableName extends PublicTableNameOrOptions extends { schema: keyof Database }
    ? keyof Database[PublicTableNameOrOptions["schema"]]["Tables"]
    : never = never,
> = PublicTableNameOrOptions extends { schema: keyof Database }
  ? Database[PublicTableNameOrOptions["schema"]]["Tables"][TableName] extends {
      Insert: infer I
    }
    ? I
    : never
  : PublicTableNameOrOptions extends keyof PublicSchema["Tables"]
    ? PublicSchema["Tables"][PublicTableNameOrOptions] extends {
        Insert: infer I
      }
      ? I
      : never
    : never

export type TablesUpdate<
  PublicTableNameOrOptions extends
    | keyof PublicSchema["Tables"]
    | { schema: keyof Database },
  TableName extends PublicTableNameOrOptions extends { schema: keyof Database }
    ? keyof Database[PublicTableNameOrOptions["schema"]]["Tables"]
    : never = never,
> = PublicTableNameOrOptions extends { schema: keyof Database }
  ? Database[PublicTableNameOrOptions["schema"]]["Tables"][TableName] extends {
      Update: infer U
    }
    ? U
    : never
  : PublicTableNameOrOptions extends keyof PublicSchema["Tables"]
    ? PublicSchema["Tables"][PublicTableNameOrOptions] extends {
        Update: infer U
      }
      ? U
      : never
    : never

export type Enums<
  PublicEnumNameOrOptions extends
    | keyof PublicSchema["Enums"]
    | { schema: keyof Database },
  EnumName extends PublicEnumNameOrOptions extends { schema: keyof Database }
    ? keyof Database[PublicEnumNameOrOptions["schema"]]["Enums"]
    : never = never,
> = PublicEnumNameOrOptions extends { schema: keyof Database }
  ? Database[PublicEnumNameOrOptions["schema"]]["Enums"][EnumName]
  : PublicEnumNameOrOptions extends keyof PublicSchema["Enums"]
    ? PublicSchema["Enums"][PublicEnumNameOrOptions]
    : never
