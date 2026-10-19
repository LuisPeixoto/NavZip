// Copyright 2026 NavZip contributors. MPL-2.0.
// Included once, at the end of new_tab_page_handler.cc.

void NewTabPageHandler::GetNavZipPreferences(
    GetNavZipPreferencesCallback callback) {
  NavZipStartLegacyImport();
  auto result = new_tab_page::mojom::NavZipPreferences::New();
  const auto& saved = profile_->GetPrefs()->GetDict("navzip.new_tab");
  if (const auto* shortcuts = saved.FindList("shortcuts")) {
    for (const auto& item : *shortcuts) {
      if (!item.is_dict())
        continue;
      const auto& entry = item.GetDict();
      const auto* id = entry.FindString("id");
      const auto* title = entry.FindString("title");
      const auto* url = entry.FindString("url");
      if (!id || !title || !url)
        continue;
      // Retire the old bundled shortcuts, including copies saved in profiles.
      // User-created shortcuts use UUIDs and remain untouched.
      if (*id == "github" || *id == "youtube" || *id == "figma" ||
          *id == "gmail" || *id == "chatgpt")
        continue;
      result->shortcuts.push_back(
          new_tab_page::mojom::NavZipShortcut::New(*id, *title, *url));
    }
  }
  std::move(callback).Run(std::move(result));
}

void NewTabPageHandler::SetNavZipPreferences(
    new_tab_page::mojom::NavZipPreferencesPtr preferences,
    SetNavZipPreferencesCallback callback) {
  if (profile_->IsOffTheRecord() || preferences->shortcuts.size() > 12) {
    std::move(callback).Run(false);
    return;
  }
  base::ListValue shortcuts;
  base::flat_set<std::string> ids;
  for (const auto& entry : preferences->shortcuts) {
    GURL url(entry->url);
    if (entry->id.empty() || entry->id.size() > 128 ||
        !ids.insert(entry->id).second || entry->title.empty() ||
        entry->title.size() > 256 || entry->url.size() > 2048 ||
        !url.is_valid() || !url.SchemeIsHTTPOrHTTPS() || url.has_username() ||
        url.has_password()) {
      std::move(callback).Run(false);
      return;
    }
    shortcuts.Append(base::DictValue()
                         .Set("id", entry->id)
                         .Set("title", entry->title)
                         .Set("url", url.spec()));
  }
  base::DictValue saved;
  saved.Set("shortcuts", std::move(shortcuts));
  profile_->GetPrefs()->SetDict("navzip.new_tab", std::move(saved));
  std::move(callback).Run(true);
}

void NewTabPageHandler::SetNavZipVerticalTabs(
    bool enabled,
    SetNavZipVerticalTabsCallback callback) {
  auto* browser = webui::GetBrowserWindowInterface(web_contents_);
  auto* controller =
      browser ? tabs::VerticalTabStripStateController::From(browser) : nullptr;
  if (!controller) {
    std::move(callback).Run(false);
    return;
  }
  controller->SetVerticalTabsEnabled(enabled);
  std::move(callback).Run(controller->ShouldDisplayVerticalTabs() == enabled);
}

void NewTabPageHandler::NewNavZipSplit(NewNavZipSplitCallback callback) {
  auto* browser = webui::GetBrowserWindowInterface(web_contents_);
  const bool accepted = browser &&
                        chrome::IsCommandEnabled(browser, IDC_NEW_SPLIT_TAB) &&
                        chrome::ExecuteCommand(browser, IDC_NEW_SPLIT_TAB);
  std::move(callback).Run(accepted);
}
