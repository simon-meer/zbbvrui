import {bootstrapApplication} from "@angular/platform-browser";
import {appConfig} from "./app/app.config";
import {AppComponent} from "./app/app.component";
import {mergeConfig} from "@sbb-esta/lyne-elements/core/config.js";

mergeConfig({
    icon: {
        namespaces: new Map<string, string>().set("default", "icons/")
    }
})

bootstrapApplication(AppComponent, appConfig).catch((err) =>
    console.error(err),
);
