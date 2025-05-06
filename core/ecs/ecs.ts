type ComponentMap<CT> = {
  uid: (number | null)[];
} & {
  [K in keyof CT]?: Array<CT[K] | null>;
};

namespace ECS {
  export type System<ComponentsType extends Record<string | number, any>> = (
    arg: ComponentsType & { uid: (number | null)[] },
  ) => Partial<ComponentsType> | null;
}

class ECS<ComponentsType extends Record<string | number, any>> {
  private components: ComponentMap<ComponentsType> = { uid: [] };
  private componentDefaults: Record<
    string,
    null | ComponentsType[keyof ComponentsType]
  > = { uid: null };
  private systems: Record<string, ECS.System<ComponentsType>> = {};

  /**
   * System
   */
  runSystems() {
    const limit = this.components.uid.length;
    for (let i = 0; i < limit; i++) {
      const obj = {} as Record<string, unknown>;

      for (const key in this.components) {
        obj[key] = this.components[key]![i];
      }

      const systems = Object.values(this.systems);
      for (let j = 0; j < systems.length; j++) {
        const res = systems[j](
          obj as ComponentsType & { uid: (number | null)[] },
        );
        if (!res) continue;

        for (const key in res) {
          if (!this.components[key]) continue;
          this.components[key][i] = res[key]!;
        }
      }
    }
  }

  addSystem(system: ECS.System<ComponentsType>) {
    const id = crypto.randomUUID();
    this.systems[id] = system;
    return id;
  }

  removeSystem(id: string) {
    delete this.systems[id];
  }

  getById(uid: number) {
    if (this.components.uid.length <= uid) return {};

    const obj = {} as Record<string, unknown>;

    for (const key in this.components) {
      obj[key] = this.components[key]![uid];
    }

    return obj;
  }

  /**
   * Entity
   */
  addEntity(components: ComponentsType) {
    for (const key in this.components) {
      if (key === "uid") {
        this.components[key].push(this.components[key].length);
        continue;
      }

      this.components[key]!.push(
        components[key] ?? this.componentDefaults[key],
      );
    }

    return this.components.uid.length - 1;
  }

  /**
   * Component
   */
  addComponent(
    name: keyof ComponentsType,
    fillValue = null as null | ComponentsType[keyof ComponentsType],
    defaultValue = null as null | ComponentsType[keyof ComponentsType],
  ) {
    if (this.components[name]) throw new Error("Component already exists");

    this.components[name] = new Array(this.components.uid.length).fill(
      fillValue,
    ) as ComponentMap<ComponentsType>[keyof ComponentsType];

    this.componentDefaults[name as string] = defaultValue;
  }

  removeComponent(name: keyof ComponentsType) {
    delete this.components[name];
  }
}

export { ECS };
