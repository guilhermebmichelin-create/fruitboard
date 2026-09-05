CREATE TABLE app_settings (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    startup_view TEXT NOT NULL DEFAULT 'home'
        CHECK (startup_view IN ('home', 'library', 'board', 'preferences'))
) STRICT;

INSERT INTO app_settings (singleton) VALUES (1);
