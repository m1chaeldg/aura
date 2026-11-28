//! Security UI state actions: OAuth 2.0 options, additional auth schemes,
//! per-request & per-domain mTLS certificates, OS Keychain, and external secret vaults.

use super::AuraAppState;
use crate::preferences::AuraPreferences;
use aura_core::model::{AuthConfig, ClientCertificateConfig};
use aura_engine::{keychain_delete, keychain_get, keychain_set, OAuth2TokenResult};

impl AuraAppState {
    pub fn persist_security_preferences(&self) {
        let mut prefs = AuraPreferences::load();
        prefs.domain_client_certs = self.domain_client_certs.clone();
        prefs.vault_config = self.vault_config.clone();
        let _ = prefs.save();
    }

    pub fn set_oauth2_grant_type(&mut self, grant: &str) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            if let AuthConfig::OAuth2 { grant_type, .. } = &mut tab.request.auth {
                *grant_type = grant.to_string();
                tab.mark_dirty(true);
            }
        }
    }

    pub fn toggle_oauth2_pkce(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            if let AuthConfig::OAuth2 { use_pkce, .. } = &mut tab.request.auth {
                *use_pkce = !*use_pkce;
                tab.mark_dirty(true);
            }
        }
    }

    pub fn toggle_oauth2_auto_refresh(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            if let AuthConfig::OAuth2 { auto_refresh, .. } = &mut tab.request.auth {
                *auto_refresh = !*auto_refresh;
                tab.mark_dirty(true);
            }
        }
    }

    pub fn apply_oauth2_token_result(&mut self, tab_id: &str, res: OAuth2TokenResult) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            if let AuthConfig::OAuth2 {
                access_token,
                refresh_token,
                expires_at,
                ..
            } = &mut tab.request.auth
            {
                *access_token = res.access_token;
                if let Some(rt) = res.refresh_token {
                    *refresh_token = Some(rt);
                }
                *expires_at = res.expires_at;
                tab.mark_dirty(true);
            }
        }
    }

    pub fn set_oauth1_sig_method(&mut self, method: &str) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            if let AuthConfig::OAuth1 {
                signature_method, ..
            } = &mut tab.request.auth
            {
                *signature_method = method.to_string();
                tab.mark_dirty(true);
            }
        }
    }

    pub fn toggle_oauth1_add_to_query(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            if let AuthConfig::OAuth1 { add_to_query, .. } = &mut tab.request.auth {
                *add_to_query = !*add_to_query;
                tab.mark_dirty(true);
            }
        }
    }

    pub fn set_hawk_algorithm(&mut self, algo: &str) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            if let AuthConfig::Hawk { algorithm, .. } = &mut tab.request.auth {
                *algorithm = algo.to_string();
                tab.mark_dirty(true);
            }
        }
    }

    pub fn toggle_wsse_digest(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            if let AuthConfig::Wsse {
                digest_password, ..
            } = &mut tab.request.auth
            {
                *digest_password = !*digest_password;
                tab.mark_dirty(true);
            }
        }
    }

    pub fn toggle_request_cert_enabled(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            let cert = tab
                .request
                .settings
                .client_cert
                .get_or_insert_with(ClientCertificateConfig::default);
            cert.enabled = !cert.enabled;
            tab.mark_dirty(true);
        }
    }

    pub fn add_domain_cert(&mut self) -> Result<(), String> {
        let domain = self.draft_cert_domain.trim().to_string();
        let cert_path = self.draft_cert_path.trim().to_string();
        if domain.is_empty() || cert_path.is_empty() {
            return Err("Both domain pattern and certificate path are required".to_string());
        }
        let key_path = if self.draft_cert_key_path.trim().is_empty() {
            None
        } else {
            Some(self.draft_cert_key_path.trim().to_string())
        };
        let passphrase = if self.draft_cert_passphrase.is_empty() {
            None
        } else {
            Some(self.draft_cert_passphrase.clone())
        };
        self.domain_client_certs.push(ClientCertificateConfig {
            enabled: true,
            domain: Some(domain),
            cert_path,
            key_path,
            passphrase,
        });
        self.draft_cert_domain.clear();
        self.draft_cert_path.clear();
        self.draft_cert_key_path.clear();
        self.draft_cert_passphrase.clear();
        self.persist_security_preferences();
        Ok(())
    }

    pub fn toggle_domain_cert(&mut self, idx: usize) {
        if let Some(cert) = self.domain_client_certs.get_mut(idx) {
            cert.enabled = !cert.enabled;
            self.persist_security_preferences();
        }
    }

    pub fn remove_domain_cert(&mut self, idx: usize) {
        if idx < self.domain_client_certs.len() {
            self.domain_client_certs.remove(idx);
            self.persist_security_preferences();
        }
    }

    pub fn store_keychain_entry(&mut self) -> Result<String, String> {
        let svc = self.keychain_service.trim();
        let key = self.keychain_key.trim();
        if key.is_empty() {
            return Err("Keychain key name is required".to_string());
        }
        keychain_set(svc, key, &self.keychain_value).map_err(|e| format!("{e}"))?;
        let eff_svc = if svc.is_empty() { "aura" } else { svc };
        Ok(format!("keyring://{eff_svc}/{key}"))
    }

    pub fn load_keychain_entry(&mut self) -> Result<(), String> {
        let svc = self.keychain_service.trim();
        let key = self.keychain_key.trim();
        if key.is_empty() {
            return Err("Keychain key name is required".to_string());
        }
        match keychain_get(svc, key) {
            Ok(Some(v)) => {
                self.keychain_value = v;
                Ok(())
            }
            Ok(None) => Err(format!("No OS keychain secret found for '{key}'")),
            Err(e) => Err(format!("{e}")),
        }
    }

    pub fn delete_keychain_entry(&mut self) -> Result<(), String> {
        let svc = self.keychain_service.trim();
        let key = self.keychain_key.trim();
        if key.is_empty() {
            return Err("Keychain key name is required".to_string());
        }
        keychain_delete(svc, key).map_err(|e| format!("{e}"))?;
        self.keychain_value.clear();
        Ok(())
    }

    /// Move an environment variable's secret into the OS Keychain and replace the variable value
    /// with a `keyring://<service>/<key>` reference URI.
    pub fn store_env_var_in_keychain(&mut self, idx: usize) -> Result<String, String> {
        let Some(ref mut ed) = self.environment_editor else {
            return Err("Environment editor is not open".to_string());
        };
        let Some(var) = ed.profile.variables.get_mut(idx) else {
            return Err("Variable row out of range".to_string());
        };
        let env_svc = format!(
            "env-{}",
            ed.profile
                .name
                .to_lowercase()
                .replace(|c: char| !c.is_ascii_alphanumeric(), "_")
        );
        let key = var.key.trim().to_string();
        if key.is_empty() {
            return Err("Variable key is required before storing in Keychain".to_string());
        }
        if !var.value.starts_with("keyring://") {
            keychain_set(&env_svc, &key, &var.value).map_err(|e| format!("{e}"))?;
            var.value = format!("keyring://{env_svc}/{key}");
        }
        var.is_secret = true;
        Ok(var.value.clone())
    }

    pub fn toggle_env_var_secret(&mut self, idx: usize) {
        if let Some(ref mut ed) = self.environment_editor {
            if let Some(var) = ed.profile.variables.get_mut(idx) {
                var.is_secret = !var.is_secret;
                if !var.is_secret {
                    self.revealed_env_secret_rows.remove(&idx);
                }
            }
        }
    }

    pub fn toggle_env_var_reveal(&mut self, idx: usize) {
        if self.revealed_env_secret_rows.contains(&idx) {
            self.revealed_env_secret_rows.remove(&idx);
        } else {
            self.revealed_env_secret_rows.insert(idx);
        }
    }

    pub fn add_hashicorp_vault_profile(&mut self) -> Result<String, String> {
        let name = self.draft_vault_profile_name.trim().to_string();
        let addr = self.vault_config.vault_addr.trim().to_string();
        if name.is_empty() || addr.is_empty() {
            return Err("Profile name and Vault address are required".to_string());
        }
        let id = uuid::Uuid::new_v4().to_string();
        let token = self.vault_config.vault_token.trim().to_string();
        if !token.is_empty() {
            aura_engine::set_vault_profile_token(&id, &token).map_err(|e| format!("{e}"))?;
        }
        self.vault_config
            .hashicorp_profiles
            .push(aura_engine::HashiCorpVaultProfile {
                id,
                name: name.clone(),
                addr,
                namespace: self.vault_config.vault_namespace.clone(),
                verify_ssl: true,
            });
        self.draft_vault_profile_name.clear();
        self.vault_config.vault_token.clear();
        self.persist_security_preferences();
        aura_engine::clear_secret_cache();
        Ok(name)
    }

    pub fn remove_hashicorp_vault_profile(&mut self, idx: usize) {
        if idx < self.vault_config.hashicorp_profiles.len() {
            let removed = self.vault_config.hashicorp_profiles.remove(idx);
            let _ = aura_engine::delete_vault_profile_token(&removed.id);
            self.persist_security_preferences();
            aura_engine::clear_secret_cache();
        }
    }

    pub fn add_aws_vault_profile(&mut self) -> Result<String, String> {
        let name = self.draft_aws_profile_name.trim().to_string();
        let region = self.vault_config.aws_region.trim().to_string();
        if name.is_empty() || region.is_empty() {
            return Err("Profile name and AWS region are required".to_string());
        }
        self.vault_config
            .aws_profiles
            .push(aura_engine::AwsVaultProfile {
                id: uuid::Uuid::new_v4().to_string(),
                name: name.clone(),
                region,
                profile: self.vault_config.aws_profile.clone(),
                endpoint_override: None,
            });
        self.draft_aws_profile_name.clear();
        self.persist_security_preferences();
        aura_engine::clear_secret_cache();
        Ok(name)
    }

    pub fn remove_aws_vault_profile(&mut self, idx: usize) {
        if idx < self.vault_config.aws_profiles.len() {
            self.vault_config.aws_profiles.remove(idx);
            self.persist_security_preferences();
            aura_engine::clear_secret_cache();
        }
    }

    pub fn add_keepass_profile(&mut self) -> Result<String, String> {
        let name = self.draft_keepass_name.trim().to_string();
        let db_path = self.draft_keepass_db_path.trim().to_string();
        if name.is_empty() || db_path.is_empty() {
            return Err("Profile name and .kdbx database path are required".to_string());
        }
        let id = uuid::Uuid::new_v4().to_string();
        if !self.draft_keepass_password.is_empty() {
            aura_engine::set_keepass_profile_password(&id, &self.draft_keepass_password)
                .map_err(|e| format!("{e}"))?;
        }
        let keyfile_path = if self.draft_keepass_keyfile_path.trim().is_empty() {
            None
        } else {
            Some(self.draft_keepass_keyfile_path.trim().to_string())
        };
        self.vault_config
            .keepass_profiles
            .push(aura_engine::KeePassProfile {
                id,
                name: name.clone(),
                db_path,
                keyfile_path,
            });
        self.draft_keepass_name.clear();
        self.draft_keepass_db_path.clear();
        self.draft_keepass_keyfile_path.clear();
        self.draft_keepass_password.clear();
        self.persist_security_preferences();
        aura_engine::clear_secret_cache();
        Ok(name)
    }

    pub fn remove_keepass_profile(&mut self, idx: usize) {
        if idx < self.vault_config.keepass_profiles.len() {
            let removed = self.vault_config.keepass_profiles.remove(idx);
            let _ = aura_engine::delete_keepass_profile_password(&removed.id);
            self.persist_security_preferences();
            aura_engine::clear_secret_cache();
        }
    }
}

pub(super) enum FieldOp<'a> {
    PushChar(char),
    PopChar,
    PushStr(&'a str),
    SetStr(String),
}

fn apply_req_field(s: &mut String, op: FieldOp<'_>) {
    match op {
        FieldOp::PushChar(c) => s.push(c),
        FieldOp::PopChar => {
            s.pop();
        }
        FieldOp::PushStr(txt) => s.push_str(txt),
        FieldOp::SetStr(txt) => *s = txt,
    }
}

fn apply_opt_field(opt: &mut Option<String>, op: FieldOp<'_>) {
    match op {
        FieldOp::PushChar(c) => {
            if let Some(s) = opt {
                s.push(c);
            } else {
                *opt = Some(c.to_string());
            }
        }
        FieldOp::PopChar => {
            if let Some(s) = opt {
                s.pop();
                if s.is_empty() {
                    *opt = None;
                }
            }
        }
        FieldOp::PushStr(txt) => {
            if let Some(s) = opt {
                s.push_str(txt);
            } else if !txt.is_empty() {
                *opt = Some(txt.to_string());
            }
        }
        FieldOp::SetStr(txt) => {
            *opt = if txt.is_empty() { None } else { Some(txt) };
        }
    }
}

pub(super) fn edit_oauth2_field(auth: &mut AuthConfig, idx: usize, op: FieldOp<'_>) -> bool {
    let AuthConfig::OAuth2 {
        grant_type,
        access_token,
        token_url,
        client_id,
        client_secret,
        auth_url,
        redirect_uri,
        scope,
        username,
        password,
        ..
    } = auth
    else {
        return false;
    };
    match idx {
        0 => apply_req_field(access_token, op),
        1 => apply_req_field(grant_type, op),
        2 => apply_opt_field(token_url, op),
        3 => apply_opt_field(client_id, op),
        4 => apply_opt_field(client_secret, op),
        5 => apply_opt_field(auth_url, op),
        6 => apply_opt_field(redirect_uri, op),
        7 => apply_opt_field(scope, op),
        8 => apply_opt_field(username, op),
        _ => apply_opt_field(password, op),
    }
    true
}

pub(super) fn read_oauth2_field(auth: &AuthConfig, idx: usize) -> Option<String> {
    let AuthConfig::OAuth2 {
        grant_type,
        access_token,
        token_url,
        client_id,
        client_secret,
        auth_url,
        redirect_uri,
        scope,
        username,
        password,
        ..
    } = auth
    else {
        return None;
    };
    Some(match idx {
        0 => access_token.clone(),
        1 => grant_type.clone(),
        2 => token_url.clone().unwrap_or_default(),
        3 => client_id.clone().unwrap_or_default(),
        4 => client_secret.clone().unwrap_or_default(),
        5 => auth_url.clone().unwrap_or_default(),
        6 => redirect_uri.clone().unwrap_or_default(),
        7 => scope.clone().unwrap_or_default(),
        8 => username.clone().unwrap_or_default(),
        _ => password.clone().unwrap_or_default(),
    })
}

pub(super) fn edit_auth_scheme_field(auth: &mut AuthConfig, idx: usize, op: FieldOp<'_>) -> bool {
    match auth {
        AuthConfig::Digest {
            username,
            password,
            realm,
            nonce,
            qop,
            algorithm,
        } => match idx {
            0 => apply_req_field(username, op),
            1 => apply_req_field(password, op),
            2 => apply_opt_field(realm, op),
            3 => apply_opt_field(nonce, op),
            4 => apply_opt_field(qop, op),
            _ => apply_opt_field(algorithm, op),
        },
        AuthConfig::Ntlm {
            username,
            password,
            domain,
            workstation,
        } => match idx {
            0 => apply_req_field(username, op),
            1 => apply_req_field(password, op),
            2 => apply_req_field(domain, op),
            _ => apply_req_field(workstation, op),
        },
        AuthConfig::OAuth1 {
            consumer_key,
            consumer_secret,
            token,
            token_secret,
            callback_url,
            realm,
            ..
        } => match idx {
            0 => apply_req_field(consumer_key, op),
            1 => apply_req_field(consumer_secret, op),
            2 => apply_req_field(token, op),
            3 => apply_req_field(token_secret, op),
            4 => apply_opt_field(callback_url, op),
            _ => apply_opt_field(realm, op),
        },
        AuthConfig::Hawk {
            id,
            key,
            ext,
            app,
            dlg,
            ..
        } => match idx {
            0 => apply_req_field(id, op),
            1 => apply_req_field(key, op),
            2 => apply_opt_field(ext, op),
            3 => apply_opt_field(app, op),
            _ => apply_opt_field(dlg, op),
        },
        AuthConfig::AkamaiEdgeGrid {
            client_token,
            client_secret,
            access_token,
            max_body,
        } => match idx {
            0 => apply_req_field(client_token, op),
            1 => apply_req_field(client_secret, op),
            2 => apply_req_field(access_token, op),
            _ => {
                let mut buf = max_body.to_string();
                apply_req_field(&mut buf, op);
                *max_body = buf.trim().parse::<usize>().unwrap_or(0);
            }
        },
        AuthConfig::Wsse {
            username,
            password,
            ..
        } => match idx {
            0 => apply_req_field(username, op),
            _ => apply_req_field(password, op),
        },
        _ => return false,
    }
    true
}

pub(super) fn read_auth_scheme_field(auth: &AuthConfig, idx: usize) -> Option<String> {
    Some(match auth {
        AuthConfig::Digest {
            username,
            password,
            realm,
            nonce,
            qop,
            algorithm,
        } => match idx {
            0 => username.clone(),
            1 => password.clone(),
            2 => realm.clone().unwrap_or_default(),
            3 => nonce.clone().unwrap_or_default(),
            4 => qop.clone().unwrap_or_default(),
            _ => algorithm.clone().unwrap_or_default(),
        },
        AuthConfig::Ntlm {
            username,
            password,
            domain,
            workstation,
        } => match idx {
            0 => username.clone(),
            1 => password.clone(),
            2 => domain.clone(),
            _ => workstation.clone(),
        },
        AuthConfig::OAuth1 {
            consumer_key,
            consumer_secret,
            token,
            token_secret,
            callback_url,
            realm,
            ..
        } => match idx {
            0 => consumer_key.clone(),
            1 => consumer_secret.clone(),
            2 => token.clone(),
            3 => token_secret.clone(),
            4 => callback_url.clone().unwrap_or_default(),
            _ => realm.clone().unwrap_or_default(),
        },
        AuthConfig::Hawk {
            id,
            key,
            ext,
            app,
            dlg,
            ..
        } => match idx {
            0 => id.clone(),
            1 => key.clone(),
            2 => ext.clone().unwrap_or_default(),
            3 => app.clone().unwrap_or_default(),
            _ => dlg.clone().unwrap_or_default(),
        },
        AuthConfig::AkamaiEdgeGrid {
            client_token,
            client_secret,
            access_token,
            max_body,
        } => match idx {
            0 => client_token.clone(),
            1 => client_secret.clone(),
            2 => access_token.clone(),
            _ => max_body.to_string(),
        },
        AuthConfig::Wsse {
            username,
            password,
            ..
        } => match idx {
            0 => username.clone(),
            _ => password.clone(),
        },
        _ => return None,
    })
}

pub(super) fn edit_request_cert_field(
    cert_opt: &mut Option<ClientCertificateConfig>,
    idx: usize,
    op: FieldOp<'_>,
) {
    let cert = cert_opt.get_or_insert_with(ClientCertificateConfig::default);
    match idx {
        0 => apply_req_field(&mut cert.cert_path, op),
        1 => apply_opt_field(&mut cert.key_path, op),
        2 => apply_opt_field(&mut cert.passphrase, op),
        _ => apply_opt_field(&mut cert.domain, op),
    }
}

pub(super) fn read_request_cert_field(cert_opt: Option<&ClientCertificateConfig>, idx: usize) -> String {
    let Some(cert) = cert_opt else {
        return String::new();
    };
    match idx {
        0 => cert.cert_path.clone(),
        1 => cert.key_path.clone().unwrap_or_default(),
        2 => cert.passphrase.clone().unwrap_or_default(),
        _ => cert.domain.clone().unwrap_or_default(),
    }
}
